use std::io::Read;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use url::Url;

use crate::repositories::git_repository::GitRepository;
use crate::services::integration_service::IntegrationService;

const PAGE_SIZE: usize = 25;
const MAX_RESPONSE_BYTES: u64 = 4 * 1024 * 1024;

#[derive(Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum Kind {
    Issue,
    PullRequest,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Request {
    pub kind: Kind,
    pub number: Option<u64>,
    pub cursor: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Credentials {
    pub gitlab: Option<String>,
    pub gitlab_authority: Option<String>,
    pub gitlab_self: Option<String>,
    pub bitbucket: Option<String>,
    pub azure: Option<String>,
    pub azure_domain: Option<String>,
}

#[derive(Serialize)]
pub struct TrackingPage {
    items: Vec<Value>,
    next: Option<String>,
}

#[derive(PartialEq, Eq)]
enum Provider {
    GitLab,
    Bitbucket,
    Azure,
}

struct Endpoint {
    provider: Provider,
    api: String,
    web: String,
    azure_project: String,
    header: Option<(&'static str, String)>,
}

fn authority(url: &Url) -> String {
    let host = url.host().map(|host| host.to_string()).unwrap_or_default();
    match url.port() {
        Some(port) => format!("{host}:{port}"),
        None => host,
    }
}

fn remote_url(raw: &str) -> Option<Url> {
    if let Ok(url) = Url::parse(raw) {
        if matches!(url.scheme(), "https" | "http" | "ssh" | "git+ssh")
            && url.query().is_none()
            && url.fragment().is_none()
        {
            return Some(url);
        }
    }
    let (_, rest) = raw.split_once('@')?;
    let (host, path) = rest.split_once(':')?;
    if host.contains(['/', '\\', '@']) || path.contains(['?', '#']) {
        return None;
    }
    Url::parse(&format!("ssh://{host}/{}", path.trim_start_matches('/'))).ok()
}

fn clean_domain(raw: &str) -> String {
    raw.trim()
        .trim_start_matches("https://")
        .trim_start_matches("http://")
        .trim_end_matches('/')
        .to_ascii_lowercase()
}

fn endpoint(raw: &str, credentials: &Credentials) -> Option<Endpoint> {
    let url = remote_url(raw)?;
    let host = url.host_str()?;
    let remote_authority = authority(&url).to_ascii_lowercase();
    let mut parts: Vec<_> = url
        .path_segments()?
        .filter(|part| !part.is_empty())
        .collect();
    let last = parts.last_mut()?;
    *last = last.strip_suffix(".git").unwrap_or(last);
    if parts
        .iter()
        .any(|part| part.is_empty() || part.contains(['?', '#', '\\']))
    {
        return None;
    }
    let basic = |user: &str, token: &str| {
        (
            "Authorization",
            IntegrationService::basic_auth_header(user, token),
        )
    };

    if host == "bitbucket.org" && parts.len() == 2 {
        let header = credentials
            .bitbucket
            .as_deref()
            .filter(|token| !token.is_empty())
            .map(|token| match token.split_once(':') {
                Some((user, secret)) => basic(user, secret),
                None => ("Authorization", format!("Bearer {token}")),
            });
        let path = parts.join("/");
        return Some(Endpoint {
            provider: Provider::Bitbucket,
            api: format!("https://api.bitbucket.org/2.0/repositories/{path}"),
            web: format!("https://bitbucket.org/{path}"),
            azure_project: String::new(),
            header,
        });
    }

    let azure_parts = match (host, parts.as_slice()) {
        ("dev.azure.com", [org, project, "_git", repo])
        | ("ssh.dev.azure.com", ["v3", org, project, repo]) => Some((*org, *project, *repo)),
        (legacy, [project, "_git", repo] | ["DefaultCollection", project, "_git", repo])
            if legacy.ends_with(".visualstudio.com") =>
        {
            Some((legacy.strip_suffix(".visualstudio.com")?, *project, *repo))
        }
        _ => None,
    };
    if let Some((org, project, repo)) = azure_parts {
        let approved = credentials.azure_domain.as_deref().map(clean_domain)?;
        if !approved.eq_ignore_ascii_case(&format!("dev.azure.com/{org}"))
            && !approved.eq_ignore_ascii_case(&format!("{org}.visualstudio.com"))
        {
            return None;
        }
        let header = credentials
            .azure
            .as_deref()
            .filter(|token| !token.is_empty())
            .map(|token| basic("pat", token));
        let project = format!("https://dev.azure.com/{org}/{project}");
        return Some(Endpoint {
            provider: Provider::Azure,
            api: format!("{project}/_apis/git/repositories/{repo}"),
            web: format!("{project}/_git/{repo}"),
            azure_project: project,
            header,
        });
    }

    let hosted_gitlab = host == "gitlab.com";
    let self_managed = credentials
        .gitlab_authority
        .as_deref()
        .map(clean_domain)
        .and_then(|domain| Url::parse(&format!("https://{domain}")).ok())
        .filter(|configured| {
            configured.path() == "/"
                && configured.username().is_empty()
                && configured.password().is_none()
                && configured.query().is_none()
                && configured.fragment().is_none()
        })
        .filter(|configured| {
            if matches!(url.scheme(), "ssh" | "git+ssh") {
                configured.host_str().is_some_and(|host| {
                    url.host_str()
                        .is_some_and(|remote| remote.eq_ignore_ascii_case(host))
                })
            } else {
                authority(configured) == remote_authority
            }
        });
    if parts.len() >= 2 && (hosted_gitlab || self_managed.is_some()) {
        let token = if hosted_gitlab {
            credentials.gitlab.as_ref()
        } else {
            credentials.gitlab_self.as_ref()
        };
        let header = token
            .filter(|token| !token.is_empty())
            .map(|token| ("PRIVATE-TOKEN", token.clone()));
        let root = if hosted_gitlab {
            "https://gitlab.com".into()
        } else {
            format!("https://{}", authority(self_managed.as_ref()?))
        };
        return Some(Endpoint {
            provider: Provider::GitLab,
            api: format!("{root}/api/v4/projects/{}", parts.join("%2F")),
            web: format!("{root}/{}", parts.join("/")),
            azure_project: String::new(),
            header,
        });
    }
    None
}

fn text(value: &Value) -> &str {
    value.as_str().unwrap_or_default()
}
fn read_json(
    agent: &ureq::Agent,
    endpoint: &Endpoint,
    url: &str,
    body: Option<Value>,
) -> Result<Value, String> {
    let mut request = agent
        .request(if body.is_some() { "POST" } else { "GET" }, url)
        .set("Accept", "application/json");
    if let Some((name, value)) = &endpoint.header {
        request = request.set(name, value);
    }
    let response = match body {
        Some(body) => request.send_json(body),
        None => request.call(),
    };
    let response = response.map_err(|error| match error {
        ureq::Error::Status(401 | 403, _) => "Access denied. Check the configured provider token and repository or project permissions.".into(),
        ureq::Error::Status(404, _) => "The provider could not find this resource with the current credentials.".into(),
        ureq::Error::Status(status, _) => format!("The provider returned HTTP {status}."),
        ureq::Error::Transport(_) => "Could not connect to the provider. Check the network or server certificate.".into(),
    })?;
    if !(200..300).contains(&response.status()) {
        return Err("The provider redirected the request. Verify the remote URL and provider configuration.".into());
    }
    serde_json::from_reader(response.into_reader().take(MAX_RESPONSE_BYTES))
        .map_err(|_| "The provider response is too large or not valid JSON.".into())
}

fn number(value: &Value) -> Option<u64> {
    value
        .as_u64()
        .filter(|number| *number > 0 && *number <= i32::MAX as u64)
}

fn map_item(endpoint: &Endpoint, kind: Kind, item: &Value, detail: bool) -> Option<Value> {
    let bound = |value: &Value| {
        text(value)
            .chars()
            .take(if detail { 48_000 } else { 220 })
            .collect::<String>()
    };
    let branch = |value: &Value| text(value).trim_start_matches("refs/heads/").to_owned();
    let push_label =
        |label: &Value| json!({ "name": text(label), "color": "64748b", "description": "" });
    let pull = kind == Kind::PullRequest;
    let title = |value: &Value| {
        value
            .as_str()
            .map(|value| value.chars().take(512).collect::<String>())
    };
    match endpoint.provider {
        Provider::GitLab => {
            let id = number(&item["iid"])?;
            let state = match text(&item["state"]) {
                "opened" => "open",
                state => state,
            };
            Some(
                json!({ "id": id, "number": id, "title": title(&item["title"])?, "description": bound(&item["description"]), "state": state,
                    "url": format!("{}/-/{}/{}", endpoint.web, if pull { "merge_requests" } else { "issues" }, id),
                    "author": text(&item["author"]["username"]), "createdAt": text(&item["created_at"]), "updatedAt": text(&item["updated_at"]),
                    "labels": item["labels"].as_array().into_iter().flatten().map(push_label).collect::<Vec<_>>(),
                    "assignees": item["assignees"].as_array().into_iter().flatten().map(|user| text(&user["username"])).collect::<Vec<_>>(),
                    "comments": item["user_notes_count"], "draft": item["draft"].as_bool().unwrap_or(false), "merged": state == "merged",
                    "sourceBranch": text(&item["source_branch"]), "targetBranch": text(&item["target_branch"]), "mergeableState": if text(&item["detailed_merge_status"]) == "conflict" { "dirty" } else { "" },
                }),
            )
        }
        Provider::Bitbucket => {
            let id = number(&item["id"])?;
            let state = text(&item["state"]).to_ascii_lowercase();
            Some(
                json!({ "id": id, "number": id, "title": title(&item["title"])?, "description": bound(&item["description"]),
                    "state": state, "url": format!("{}/pull-requests/{}", endpoint.web, id), "author": text(&item["author"]["display_name"]),
                    "createdAt": text(&item["created_on"]), "updatedAt": text(&item["updated_on"]), "comments": item["comment_count"],
                    "draft": item["draft"].as_bool().unwrap_or(false), "merged": state == "merged", "sourceBranch": text(&item["source"]["branch"]["name"]), "targetBranch": text(&item["destination"]["branch"]["name"]),
                }),
            )
        }
        Provider::Azure if pull => {
            let id = number(&item["pullRequestId"])?;
            let state = match text(&item["status"]) {
                "active" => "open",
                "completed" => "merged",
                "abandoned" => "closed",
                _ => "unknown",
            };
            Some(
                json!({ "id": id, "number": id, "title": title(&item["title"])?, "description": bound(&item["description"]), "state": state,
                    "url": format!("{}/pullrequest/{}", endpoint.web, id), "author": text(&item["createdBy"]["displayName"]),
                    "createdAt": text(&item["creationDate"]), "updatedAt": text(&item["closedDate"]), "draft": item["isDraft"].as_bool().unwrap_or(false), "merged": state == "merged",
                    "sourceBranch": branch(&item["sourceRefName"]), "targetBranch": branch(&item["targetRefName"]),
                }),
            )
        }
        Provider::Azure => {
            let id = number(&item["id"])?;
            let fields = &item["fields"];
            let tags: Vec<_> = text(&fields["System.Tags"])
                .split(';')
                .map(str::trim)
                .filter(|tag| !tag.is_empty())
                .map(|tag| json!({ "name": tag, "description": "", "color": "64748b" }))
                .collect();
            Some(
                json!({ "id": id, "number": id, "title": title(&fields["System.Title"])?, "description": bound(&fields["System.Description"]),
                    "state": text(&fields["System.State"]), "url": format!("{}/_workitems/edit/{}", endpoint.azure_project, id), "author": text(&fields["System.CreatedBy"]["displayName"]),
                    "createdAt": text(&fields["System.CreatedDate"]), "updatedAt": text(&fields["System.ChangedDate"]), "labels": tags,
                    "assignees": fields["System.AssignedTo"]["displayName"].as_str().map(|name| vec![name]).unwrap_or_default(),
                }),
            )
        }
    }
}

fn list_or_detail(
    endpoint: &Endpoint,
    agent: &ureq::Agent,
    input: Request,
) -> Result<TrackingPage, String> {
    let detail = input.number.is_some();
    let pull = input.kind == Kind::PullRequest;
    if input
        .number
        .is_some_and(|id| id == 0 || id > i32::MAX as u64)
    {
        return Err("Invalid work item number.".into());
    }
    let cursor = input
        .cursor
        .as_deref()
        .map(|value| value.parse::<u32>().ok())
        .unwrap_or(Some(0))
        .ok_or("Invalid page cursor.")?;
    let page = cursor.checked_add(1).ok_or("Invalid page cursor.")?;
    let base = &endpoint.api;
    let project = &endpoint.azure_project;
    let mut next = None;
    let data = match (&endpoint.provider, pull, input.number) {
        (Provider::Bitbucket, false, _) => {
            return Err("Bitbucket Cloud no longer provides a native issue tracker.".into())
        }
        (Provider::GitLab, _, id) => {
            let resource = if pull { "merge_requests" } else { "issues" };
            let url = if let Some(id) = id {
                format!("{base}/{resource}/{id}")
            } else {
                format!("{base}/{resource}?state=opened&scope=all&per_page={PAGE_SIZE}&page={page}")
            };
            let result = read_json(agent, endpoint, &url, None)?;
            if result
                .as_array()
                .is_some_and(|items| items.len() == PAGE_SIZE)
            {
                next = Some(page.to_string());
            }
            result
        }
        (Provider::Bitbucket, true, id) => {
            let url = if let Some(id) = id {
                format!("{base}/pullrequests/{id}")
            } else {
                format!("{base}/pullrequests?state=OPEN&pagelen={PAGE_SIZE}&page={page}")
            };
            let mut result = read_json(agent, endpoint, &url, None)?;
            if detail {
                result
            } else {
                if result["next"].is_string() {
                    next = Some(page.to_string());
                }
                result["values"].take()
            }
        }
        (Provider::Azure, true, id) => {
            let mut result = if let Some(id) = id {
                read_json(
                    agent,
                    endpoint,
                    &format!("{base}/pullrequests/{id}?api-version=7.1"),
                    None,
                )?
            } else {
                read_json(agent, endpoint, &format!("{base}/pullrequests?searchCriteria.status=active&$top={PAGE_SIZE}&$skip={cursor}&api-version=7.1"), None)?
            };
            if detail {
                result
            } else {
                if result["value"]
                    .as_array()
                    .is_some_and(|items| items.len() == PAGE_SIZE)
                {
                    next = cursor
                        .checked_add(PAGE_SIZE as u32)
                        .map(|value| value.to_string());
                }
                result["value"].take()
            }
        }
        (Provider::Azure, false, Some(id)) => read_json(
            agent,
            endpoint,
            &format!("{project}/_apis/wit/workitems/{id}?api-version=7.1"),
            None,
        )?,
        (Provider::Azure, false, None) => {
            let before = if cursor == 0 {
                String::new()
            } else {
                format!(" AND [System.Id] < {cursor}")
            };
            let query = format!("SELECT [System.Id] FROM WorkItems WHERE [System.TeamProject] = @project{before} ORDER BY [System.Id] DESC");
            let result = read_json(
                agent,
                endpoint,
                &format!("{project}/_apis/wit/wiql?$top={PAGE_SIZE}&api-version=7.1"),
                Some(json!({ "query": query })),
            )?;
            let ids: Vec<_> = result["workItems"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|value| number(&value["id"]))
                .take(PAGE_SIZE)
                .collect();
            if ids.is_empty() {
                return Ok(TrackingPage {
                    items: Vec::new(),
                    next,
                });
            }
            if ids.len() == PAGE_SIZE {
                next = ids.iter().min().map(|value| value.to_string());
            }
            let ids: Vec<_> = ids.into_iter().map(|id| id.to_string()).collect();
            let mut result = read_json(agent, endpoint, &format!("{project}/_apis/wit/workitems?ids={}&fields=System.Id,System.Title,System.State,System.CreatedBy,System.CreatedDate,System.ChangedDate,System.AssignedTo,System.Tags&errorPolicy=Omit&api-version=7.1", ids.join(",")), None)?;
            result["value"].take()
        }
    };
    let items = if detail {
        map_item(endpoint, input.kind, &data, true)
            .into_iter()
            .collect()
    } else {
        data.as_array()
            .ok_or("The provider returned an unexpected tracking list.")?
            .iter()
            .take(PAGE_SIZE)
            .filter_map(|item| map_item(endpoint, input.kind, item, false))
            .collect()
    };
    Ok(TrackingPage { items, next })
}

pub fn track(
    path: &str,
    remote: &str,
    credentials: Credentials,
    input: Request,
) -> Result<TrackingPage, String> {
    let repo = GitRepository::open(path)?;
    let remote = repo
        .find_remote(remote)
        .map_err(|_| "The selected remote no longer exists.")?;
    let endpoint = endpoint(
        remote.url().ok_or("The selected remote has no URL.")?,
        &credentials,
    )
    .ok_or("Configure this provider and its server or organisation in Options first.")?;
    let agent = ureq::AgentBuilder::new()
        .redirects(0)
        .timeout(Duration::from_secs(10))
        .build();
    list_or_detail(&endpoint, &agent, input)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{BufRead, BufReader, Write};
    use std::net::TcpListener;

    fn credentials() -> Credentials {
        serde_json::from_value(json!({ "gitlab": "cloud", "gitlabAuthority": "git.example.invalid:8443", "gitlabSelf": "self", "bitbucket": "user@example.invalid:bb", "azure": "pat", "azureDomain": "dev.azure.com/acme" })).unwrap()
    }

    #[test]
    fn maps_remote_urls_without_sending_credentials_to_the_wrong_host() {
        let auth = credentials();
        let cloud = endpoint("git@gitlab.com:parent/subgroup/project.git", &auth).unwrap();
        assert!(cloud
            .api
            .ends_with("/api/v4/projects/parent%2Fsubgroup%2Fproject"));
        assert_eq!(cloud.header.unwrap(), ("PRIVATE-TOKEN", "cloud".into()));
        let self_hosted = endpoint(
            "ssh://git@git.example.invalid:2222/group/project.git",
            &auth,
        )
        .unwrap();
        assert_eq!(
            self_hosted.api,
            "https://git.example.invalid:8443/api/v4/projects/group%2Fproject"
        );
        assert_eq!(
            self_hosted.header.unwrap(),
            ("PRIVATE-TOKEN", "self".into())
        );
        assert!(endpoint("https://git.example.invalid:1443/group/project.git", &auth).is_none());
        assert!(endpoint(
            "https://git.example.invalid.attacker.invalid/group/project.git",
            &auth
        )
        .is_none());
        assert!(endpoint(
            "https://git.example.invalid:8443@attacker.invalid/group/project.git",
            &auth
        )
        .is_none());
        let azure = endpoint("git@ssh.dev.azure.com:v3/acme/Team/My%20Repo", &auth).unwrap();
        assert_eq!(
            azure.api,
            "https://dev.azure.com/acme/Team/_apis/git/repositories/My%20Repo"
        );
        assert!(endpoint("https://other.visualstudio.com/Team/_git/Repo", &auth).is_none());
        let bitbucket = endpoint("git@bitbucket.org:team/repo.git", &auth).unwrap();
        assert_eq!(
            bitbucket.api,
            "https://api.bitbucket.org/2.0/repositories/team/repo"
        );
        assert!(bitbucket.header.as_ref().unwrap().1.starts_with("Basic "));
        assert!(list_or_detail(
            &bitbucket,
            &ureq::agent(),
            Request {
                kind: Kind::Issue,
                number: None,
                cursor: None
            }
        )
        .err()
        .unwrap()
        .contains("no longer"));
    }

    fn mock(responses: Vec<(u16, Value)>) -> (String, std::thread::JoinHandle<Vec<String>>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let handle = std::thread::spawn(move || {
            responses.into_iter().map(|(status, body)| {
                let (mut stream, _) = listener.accept().unwrap();
                stream.set_read_timeout(Some(Duration::from_secs(3))).unwrap();
                let mut reader = BufReader::new(&mut stream);
                let mut request = String::new();
                let mut content_len = 0;
                loop {
                    let mut line = String::new();
                    assert_ne!(reader.read_line(&mut line).unwrap(), 0);
                    if let Some((name, value)) = line.split_once(':') {
                        if name.eq_ignore_ascii_case("content-length") { content_len = value.trim().parse().unwrap(); }
                    }
                    request.push_str(&line);
                    if line == "\r\n" { break; }
                }
                let mut bytes = vec![0; content_len];
                reader.read_exact(&mut bytes).unwrap();
                request.push_str(&String::from_utf8_lossy(&bytes));
                let body = body.to_string();
                write!(stream, "HTTP/1.1 {status} Test\r\nConnection: close\r\nContent-Length: {}\r\nLocation: https://foreign.invalid/unsafe\r\n\r\n{body}", body.len()).unwrap();
                request
            }).collect()
        });
        (url, handle)
    }

    #[test]
    fn fetches_azure_work_items_by_project_and_maps_the_list() {
        let (url, requests) = mock(vec![
            (200, json!({"workItems":[{"id":19}]})),
            (
                200,
                json!({"value":[{"id":19,"fields":{"System.Title":"Fix login","System.State":"Active"}}]}),
            ),
        ]);
        let mut azure = endpoint(
            "https://dev.azure.com/acme/Project/_git/Repo",
            &credentials(),
        )
        .unwrap();
        azure.azure_project = format!("{url}/acme/Project");
        let page = list_or_detail(
            &azure,
            &ureq::agent(),
            Request {
                kind: Kind::Issue,
                number: None,
                cursor: Some("20".into()),
            },
        )
        .unwrap();
        assert_eq!(page.items[0]["number"], 19);
        assert_eq!(page.items[0]["title"], "Fix login");
        assert!(page.next.is_none());
        let requests = requests.join().unwrap();
        assert!(requests[0].contains("POST /acme/Project/_apis/wit/wiql?$top=25"));
        assert!(requests[0].contains("[System.TeamProject] = @project AND [System.Id] < 20"));
        assert!(requests[1].contains("GET /acme/Project/_apis/wit/workitems?ids=19"));
    }

    #[test]
    fn uses_gitlab_project_iids_and_refuses_redirects() {
        let (url, requests) = mock(vec![
            (
                200,
                json!({"iid":6,"id":313,"title":"Merge feature","state":"opened","source_branch":"feature/login","target_branch":"main"}),
            ),
            (302, json!({})),
        ]);
        let mut gitlab = endpoint("https://gitlab.com/group/repo", &credentials()).unwrap();
        gitlab.api = format!("{url}/api/v4/projects/group%2Frepo");
        let agent = ureq::AgentBuilder::new()
            .redirects(0)
            .timeout(Duration::from_secs(3))
            .build();
        let request = || Request {
            kind: Kind::PullRequest,
            number: Some(6),
            cursor: None,
        };
        let result = list_or_detail(&gitlab, &agent, request()).unwrap();
        assert_eq!(result.items[0]["number"], 6);
        assert_eq!(result.items[0]["state"], "open");
        assert_eq!(
            result.items[0]["url"],
            "https://gitlab.com/group/repo/-/merge_requests/6"
        );
        assert!(list_or_detail(&gitlab, &agent, request())
            .err()
            .unwrap()
            .contains("redirected"));
        let requests = requests.join().unwrap();
        assert!(requests[0].contains("GET /api/v4/projects/group%2Frepo/merge_requests/6"));
        assert!(requests[0]
            .to_ascii_lowercase()
            .contains("private-token: cloud"));
    }
}

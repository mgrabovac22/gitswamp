const REPO = "mgrabovac22/gitswamp";
const RELEASE_API = `https://api.github.com/repos/${REPO}/releases/latest`;
const REPO_API = `https://api.github.com/repos/${REPO}`;
const RELEASE_PAGE = `https://github.com/${REPO}/releases/latest`;
const RELEASES_PAGE = `https://github.com/${REPO}/releases`;

const PLATFORMS = {
  windows: {
    icon: "i-windows",
    label: "Windows",
    detail: "x64 installer",
    short: "Windows",
    toast: "Run the .exe installer to finish setting up GitSwamp.",
  },
  "mac-arm": {
    icon: "i-apple",
    label: "macOS",
    detail: "Apple Silicon / arm64",
    short: "macOS Apple Silicon",
    toast: "Open the .dmg and drag GitSwamp into Applications.",
  },
  "mac-intel": {
    icon: "i-apple",
    label: "macOS",
    detail: "Intel / x64",
    short: "macOS Intel",
    toast: "Open the .dmg and drag GitSwamp into Applications.",
  },
  linux: {
    icon: "i-linux",
    label: "Linux",
    detail: "Debian package / amd64",
    short: "Linux .deb",
    toast: "Install with sudo apt install ~/Downloads/<file>, or open it in your package manager.",
  },
  "linux-appimage": {
    icon: "i-linux",
    label: "Linux",
    detail: "Universal AppImage",
    short: "Linux AppImage",
    toast: "Make the AppImage executable, then run it. No installation needed.",
  },
};

const ORDER = ["windows", "mac-arm", "mac-intel", "linux", "linux-appimage"];
const state = {
  release: null,
  builds: {},
  recommended: null,
  mobile: false,
  android: false,
};

const qs = (selector, parent = document) => parent.querySelector(selector);
const qsa = (selector, parent = document) => Array.from(parent.querySelectorAll(selector));

const nodes = {
  header: qs("#siteHeader"),
  navLinks: qs("#navLinks"),
  menuButton: qs("#menuButton"),
  progress: qs("#scrollProgress"),
  glow: qs("#cursorGlow"),
  primary: qs("#primaryDownload"),
  secondary: qs("#secondaryDownload"),
  primaryLabel: qs("#primaryLabel"),
  primarySub: qs("#primarySub"),
  secondaryLabel: qs("#secondaryLabel"),
  secondarySub: qs("#secondarySub"),
  toggle: qs("#downloadToggle"),
  menu: qs("#downloadMenu"),
  options: qs("#downloadOptions"),
  detected: qs("#detectedPlatform"),
  starCount: qs("#starCount"),
  installCommand: qs("#installCommand"),
  installFile: qs("#installFile"),
  copyCommand: qs("#copyCommand"),
  toast: qs("#toast"),
  toastTitle: qs("#toastTitle"),
  toastBody: qs("#toastBody"),
  toastClose: qs("#toastClose"),
  stage: qs("#stage"),
  year: qs("#year"),
};

let toastTimer = null;
let menuTimer = null;
let pointerFrame = null;

function formatSize(bytes) {
  if (!Number.isFinite(bytes) || bytes <= 0) return "—";
  const units = ["B", "KB", "MB", "GB"];
  let value = bytes;
  let index = 0;
  while (value >= 1024 && index < units.length - 1) {
    value /= 1024;
    index += 1;
  }
  const digits = value >= 10 ? 0 : 1;
  return `${value.toFixed(digits)} ${units[index]}`;
}

function normalizeVersion(tag) {
  if (!tag) return "latest";
  return tag.startsWith("v") ? tag : `v${tag}`;
}

function classifyAssets(assets) {
  const usable = assets.filter((asset) => {
    const name = asset.name || "";
    return !name.endsWith(".sig") && !name.endsWith(".tar.gz") && name !== "latest.json";
  });

  const find = (test) => usable.find((asset) => test(asset.name || ""));

  const windows = find((name) => /windows.*setup\.exe$/i.test(name) || (/setup\.exe$/i.test(name) && /(x64|x86_64)/i.test(name)));
  const macArm = find((name) => /\.dmg$/i.test(name) && /(aarch64|arm64)/i.test(name));
  const macIntel = find((name) => /\.dmg$/i.test(name) && /(x64|x86_64)/i.test(name) && !/(aarch64|arm64)/i.test(name));
  const linuxDeb = find((name) => /\.deb$/i.test(name) && !/(aarch64|arm64)/i.test(name)) || find((name) => /\.deb$/i.test(name));
  const linuxAppImage = find((name) => /\.AppImage$/i.test(name));

  return {
    windows,
    "mac-arm": macArm,
    "mac-intel": macIntel,
    linux: linuxDeb,
    "linux-appimage": linuxAppImage,
  };
}

function buildFromAsset(key, asset) {
  if (!asset) return null;
  return {
    key,
    name: asset.name,
    size: asset.size,
    url: asset.browser_download_url,
    meta: PLATFORMS[key],
  };
}

function macArchFromWebGL() {
  try {
    const canvas = document.createElement("canvas");
    const gl = canvas.getContext("webgl") || canvas.getContext("experimental-webgl");
    if (!gl) return "";
    const info = gl.getExtension("WEBGL_debug_renderer_info");
    const renderer = info ? String(gl.getParameter(info.UNMASKED_RENDERER_WEBGL)) : "";
    if (/apple/i.test(renderer)) return "arm";
    if (/intel|amd|radeon/i.test(renderer)) return "x86";
  } catch {
    return "";
  }
  return "";
}

async function detectPlatform() {
  const ua = navigator.userAgent || "";
  const data = navigator.userAgentData;
  const platform = (data && data.platform) || navigator.platform || "";
  const touchMac = /Macintosh/i.test(ua) && navigator.maxTouchPoints > 1;

  if (/Android/i.test(ua) || /iPhone|iPad|iPod/i.test(ua) || touchMac) {
    const android = /Android/i.test(ua);
    return {
      key: null,
      label: android ? "Android detected · test build" : "Desktop builds only",
      mobile: true,
      android,
    };
  }

  if (/Win/i.test(platform) || /Windows/i.test(ua)) {
    return { key: "windows", label: "Windows detected" };
  }

  if (/Mac/i.test(platform) || /Mac OS X/i.test(ua)) {
    let arch = "";
    try {
      if (data && typeof data.getHighEntropyValues === "function") {
        const values = await data.getHighEntropyValues(["architecture"]);
        arch = String(values.architecture || "").toLowerCase();
      }
    } catch {
      arch = "";
    }
    if (!arch) arch = macArchFromWebGL();
    if (/x86|intel|amd/.test(arch)) {
      return { key: "mac-intel", label: "macOS · Intel detected" };
    }
    if (/arm|aarch/.test(arch)) {
      return { key: "mac-arm", label: "macOS · Apple Silicon detected" };
    }
    return { key: "mac-arm", label: "macOS detected" };
  }

  if (/Linux|X11/i.test(platform) || /Linux/i.test(ua)) {
    return { key: "linux", label: "Linux detected" };
  }

  return { key: null, label: "Pick your desktop platform" };
}

function installCommandFor(build) {
  if (!build) return { file: "waiting for release…", command: "Open the downloaded file to install GitSwamp." };
  const file = build.name;
  if (build.key === "windows") {
    return { file, command: `start "" "%USERPROFILE%\\Downloads\\${file}"` };
  }
  if (build.key === "mac-arm" || build.key === "mac-intel") {
    return { file, command: `open ~/Downloads/${file}` };
  }
  if (build.key === "linux") {
    return { file, command: `sudo apt install ~/Downloads/${file}` };
  }
  return { file, command: `chmod +x ~/Downloads/${file} && ~/Downloads/${file}` };
}

function setIcon(useNode, icon) {
  if (useNode) useNode.setAttribute("href", `#${icon}`);
}

function setDownloadButton(anchor, labelNode, subNode, build) {
  if (!anchor) return;
  const iconUse = qs("use", anchor);
  if (!build) {
    anchor.href = RELEASE_PAGE;
    anchor.removeAttribute("data-build-key");
    if (labelNode) labelNode.textContent = "Get the latest release";
    if (subNode) subNode.textContent = "choose your platform";
    setIcon(iconUse, "i-download");
    return;
  }
  anchor.href = build.url;
  anchor.dataset.buildKey = build.key;
  if (labelNode) labelNode.textContent = `Download for ${build.meta.short}`;
  if (subNode) subNode.textContent = `${build.name.split(".").pop().toLowerCase()} · ${formatSize(build.size)}`;
  setIcon(iconUse, build.meta.icon);
}

function updateInstallPanel() {
  const fallback = state.recommended && state.builds[state.recommended.key]
    ? state.recommended.key
    : ORDER.find((key) => state.builds[key]);
  const build = fallback ? state.builds[fallback] : null;
  const { file, command } = installCommandFor(build);
  if (nodes.installFile) nodes.installFile.textContent = file;
  if (nodes.installCommand) nodes.installCommand.textContent = command;
}

function createOption(key, build, detected) {
  const anchor = document.createElement("a");
  anchor.className = "download-option";
  anchor.href = build.url;
  anchor.dataset.buildKey = key;

  const icon = document.createElementNS("http://www.w3.org/2000/svg", "svg");
  icon.setAttribute("class", "icon");
  const use = document.createElementNS("http://www.w3.org/2000/svg", "use");
  use.setAttribute("href", `#${build.meta.icon}`);
  icon.append(use);

  const copy = document.createElement("span");
  copy.className = "download-option-copy";
  const title = document.createElement("span");
  title.textContent = `${build.meta.label} — ${build.meta.detail}`;
  title.style.fontFamily = "var(--display)";
  title.style.fontSize = "0.8rem";
  title.style.color = "var(--cream)";
  const file = document.createElement("span");
  file.textContent = build.name;
  copy.append(title, file);

  const meta = document.createElement("span");
  meta.className = "download-option-meta";
  const size = document.createElement("span");
  size.textContent = formatSize(build.size);
  meta.append(size);
  if (detected) {
    const badge = document.createElement("span");
    badge.className = "detected";
    badge.textContent = "detected";
    meta.append(badge);
  }

  anchor.append(icon, copy, meta);
  return anchor;
}

function renderOptions() {
  if (!nodes.options) return;
  nodes.options.replaceChildren();
  const detected = state.recommended ? state.recommended.key : null;
  const available = ORDER.filter((key) => state.builds[key]);
  if (!available.length) {
    const message = document.createElement("span");
    message.className = "menu-loading";
    message.textContent = "Couldn't read the latest release. Use the releases page instead.";
    nodes.options.append(message);
    return;
  }
  const sorted = detected && state.builds[detected]
    ? [detected, ...available.filter((key) => key !== detected)]
    : available;
  sorted.forEach((key) => {
    nodes.options.append(createOption(key, state.builds[key], key === detected));
  });
}

function updatePlatformCards() {
  qsa("[data-platform]").forEach((card) => {
    const key = card.dataset.platform;
    const build = state.builds[key];
    const size = qs("[data-size]", card);
    if (build) {
      card.href = build.url;
      card.dataset.buildKey = key;
      if (size) size.textContent = formatSize(build.size);
    } else {
      card.href = RELEASES_PAGE;
      card.removeAttribute("data-build-key");
      if (size) size.textContent = "—";
    }
    card.dataset.detected = state.recommended && state.recommended.key === key ? "true" : "false";
  });
}

function applyRelease(release) {
  state.release = release;
  const assetMap = classifyAssets(release.assets || []);
  state.builds = {};
  ORDER.forEach((key) => {
    const build = buildFromAsset(key, assetMap[key]);
    if (build) state.builds[key] = build;
  });

  const version = normalizeVersion(release.tag_name);
  qsa("[data-release-version]").forEach((node) => {
    node.textContent = version;
  });

  const recommendedKey = state.recommended ? state.recommended.key : null;
  const recommendedBuild = recommendedKey ? state.builds[recommendedKey] : null;
  const fallbackKey = ORDER.find((key) => state.builds[key]);
  const primaryBuild = recommendedBuild || (fallbackKey ? state.builds[fallbackKey] : null);

  setDownloadButton(nodes.primary, nodes.primaryLabel, nodes.primarySub, primaryBuild);
  setDownloadButton(nodes.secondary, nodes.secondaryLabel, nodes.secondarySub, primaryBuild);
  updatePlatformCards();
  renderOptions();
  updateInstallPanel();
}

function showToast(title, body) {
  if (!nodes.toast) return;
  if (nodes.toastTitle) nodes.toastTitle.textContent = title;
  if (nodes.toastBody) nodes.toastBody.textContent = body;
  clearTimeout(toastTimer);
  nodes.toast.classList.add("show");
  toastTimer = setTimeout(() => nodes.toast.classList.remove("show"), 6500);
}

function announceDownload(key) {
  const build = state.builds[key];
  if (!build) return;
  const platform = build.meta;
  showToast(`${platform.short} download started`, `Latest ${build.name} · ${formatSize(build.size)}. ${platform.toast.replace("<file>", build.name)}`);
}

function positionDownloadMenu() {
  if (!nodes.menu || !nodes.toggle) return;
  const toggleRect = nodes.toggle.getBoundingClientRect();
  const menuHeight = Math.max(nodes.menu.scrollHeight, nodes.menu.getBoundingClientRect().height);
  const spaceAbove = toggleRect.top - 18;
  const spaceBelow = window.innerHeight - toggleRect.bottom - 18;
  const dropUp = menuHeight > spaceBelow && spaceAbove > spaceBelow;
  const available = Math.max(120, (dropUp ? spaceAbove : spaceBelow) - 4);
  nodes.menu.style.maxHeight = `${Math.min(520, available)}px`;
  nodes.menu.classList.toggle("drop-up", dropUp);
}

function setMenuLabel(open) {
  if (!nodes.toggle) return;
  nodes.toggle.setAttribute("aria-label", open ? "Hide download options" : "Show all download options");
}

function openMenu() {
  if (!nodes.menu || !nodes.toggle) return;
  clearTimeout(menuTimer);
  nodes.menu.hidden = false;
  nodes.toggle.setAttribute("aria-expanded", "true");
  setMenuLabel(true);
  positionDownloadMenu();
  requestAnimationFrame(() => nodes.menu.classList.add("open"));
}

function closeMenu() {
  if (!nodes.menu || !nodes.toggle) return;
  nodes.toggle.setAttribute("aria-expanded", "false");
  setMenuLabel(false);
  nodes.menu.classList.remove("open");
  menuTimer = setTimeout(() => {
    if (nodes.toggle.getAttribute("aria-expanded") === "false") nodes.menu.hidden = true;
  }, 220);
}

function toggleMenu() {
  if (!nodes.menu || !nodes.toggle) return;
  if (nodes.toggle.getAttribute("aria-expanded") === "true") closeMenu();
  else openMenu();
}

async function fetchRelease() {
  const response = await fetch(RELEASE_API, {
    headers: { Accept: "application/vnd.github+json" },
    cache: "no-store",
  });
  if (!response.ok) throw new Error(`Release request failed: ${response.status}`);
  return response.json();
}

async function fetchStars() {
  try {
    const response = await fetch(REPO_API, {
      headers: { Accept: "application/vnd.github+json" },
      cache: "no-store",
    });
    if (!response.ok) return;
    const repo = await response.json();
    if (nodes.starCount && Number.isFinite(repo.stargazers_count)) {
      const formatter = new Intl.NumberFormat("en", { notation: "compact", maximumFractionDigits: 1 });
      nodes.starCount.textContent = formatter.format(repo.stargazers_count);
    }
  } catch {
    if (nodes.starCount) nodes.starCount.textContent = "—";
  }
}

function applyMobileState() {
  if (!state.mobile) return;

  const apk = qs("#mobileDownload");
  const desktopLabel = "Choose a desktop build";
  const desktopSub = "Windows · macOS · Linux";

  if (state.android && apk && nodes.primary) {
    nodes.primary.href = apk.href;
    nodes.primary.removeAttribute("data-build-key");
    if (nodes.primaryLabel) nodes.primaryLabel.textContent = "Download Android test APK";
    if (nodes.primarySub) nodes.primarySub.textContent = "v0.1.0 · 21 MB · debug build";
    setIcon(qs("use", nodes.primary), "i-android");
  } else if (nodes.primary) {
    nodes.primary.href = RELEASES_PAGE;
    nodes.primary.removeAttribute("data-build-key");
    if (nodes.primaryLabel) nodes.primaryLabel.textContent = desktopLabel;
    if (nodes.primarySub) nodes.primarySub.textContent = desktopSub;
  }

  if (nodes.secondary) nodes.secondary.href = RELEASES_PAGE;
  if (nodes.secondaryLabel) nodes.secondaryLabel.textContent = desktopLabel;
  if (nodes.secondarySub) nodes.secondarySub.textContent = desktopSub;
}

async function initDownloads() {
  const detection = await detectPlatform();
  state.recommended = detection.key ? detection : null;
  state.mobile = Boolean(detection.mobile);
  state.android = Boolean(detection.android);

  if (nodes.detected) nodes.detected.textContent = detection.label;

  try {
    const release = await fetchRelease();
    applyRelease(release);
  } catch {
    qsa("[data-release-version]").forEach((node) => {
      node.textContent = "latest";
    });
    if (nodes.primaryLabel) nodes.primaryLabel.textContent = "Get the latest release";
    if (nodes.primarySub) nodes.primarySub.textContent = "on GitHub";
    if (nodes.secondaryLabel) nodes.secondaryLabel.textContent = "Get the latest release";
    if (nodes.secondarySub) nodes.secondarySub.textContent = "on GitHub";
    if (nodes.primary) nodes.primary.href = RELEASE_PAGE;
    if (nodes.secondary) nodes.secondary.href = RELEASE_PAGE;
    renderOptions();
  }

  applyMobileState();
  updateInstallPanel();
}

function initHeader() {
  const update = () => {
    const scrollTop = window.scrollY || document.documentElement.scrollTop;
    if (nodes.header) nodes.header.classList.toggle("scrolled", scrollTop > 24);
    const max = document.documentElement.scrollHeight - window.innerHeight;
    const progress = max > 0 ? Math.min(scrollTop / max, 1) : 0;
    if (nodes.progress) nodes.progress.style.transform = `scaleX(${progress})`;
  };
  update();
  window.addEventListener("scroll", update, { passive: true });
  window.addEventListener("resize", update);
}

function initNavigation() {
  if (!nodes.menuButton || !nodes.navLinks) return;
  const close = () => {
    nodes.navLinks.classList.remove("open");
    nodes.menuButton.setAttribute("aria-expanded", "false");
    nodes.menuButton.setAttribute("aria-label", "Open menu");
  };
  nodes.menuButton.addEventListener("click", () => {
    const open = !nodes.navLinks.classList.contains("open");
    nodes.navLinks.classList.toggle("open", open);
    nodes.menuButton.setAttribute("aria-expanded", String(open));
    nodes.menuButton.setAttribute("aria-label", open ? "Close menu" : "Open menu");
  });
  qsa("a", nodes.navLinks).forEach((link) => link.addEventListener("click", close));
  window.addEventListener("resize", () => {
    if (window.innerWidth > 900) close();
  });
  document.addEventListener("keydown", (event) => {
    if (event.key === "Escape") close();
  });
}

function initDownloadMenu() {
  if (!nodes.toggle) return;
  nodes.toggle.addEventListener("click", (event) => {
    event.stopPropagation();
    toggleMenu();
  });
  nodes.menu.addEventListener("click", (event) => event.stopPropagation());
  document.addEventListener("click", () => closeMenu());
  document.addEventListener("keydown", (event) => {
    if (event.key === "Escape") closeMenu();
  });
  window.addEventListener("resize", () => {
    if (nodes.toggle.getAttribute("aria-expanded") === "true") positionDownloadMenu();
  });
  if ("ResizeObserver" in window) {
    new ResizeObserver(() => {
      if (nodes.toggle.getAttribute("aria-expanded") === "true") positionDownloadMenu();
    }).observe(nodes.menu);
  }
}

function initDownloadAnnouncements() {
  document.addEventListener("click", (event) => {
    const link = event.target.closest("a[data-build-key]");
    if (!link) return;
    announceDownload(link.dataset.buildKey);
    if (link.closest("#downloadMenu")) closeMenu();
  });
}

function initReveals() {
  const reveals = qsa(".reveal");
  if (!reveals.length) return;
  if (!("IntersectionObserver" in window)) {
    reveals.forEach((element) => element.classList.add("is-visible"));
    return;
  }
  const observer = new IntersectionObserver((entries) => {
    entries.forEach((entry) => {
      if (!entry.isIntersecting) return;
      entry.target.classList.add("is-visible");
      observer.unobserve(entry.target);
    });
  }, { threshold: 0.1, rootMargin: "0px 0px -40px" });
  reveals.forEach((element) => observer.observe(element));
}

function initCounters() {
  const counters = qsa("[data-count]");
  if (!counters.length) return;
  const prefersReduced = window.matchMedia("(prefers-reduced-motion: reduce)").matches;
  if (prefersReduced) return;
  const animate = (element) => {
    const target = Number(element.dataset.count);
    if (!Number.isFinite(target) || target === 0) return;
    const duration = 1100;
    const start = performance.now();
    const tick = (now) => {
      const progress = Math.min((now - start) / duration, 1);
      const eased = 1 - Math.pow(1 - progress, 3);
      element.textContent = String(Math.round(target * eased));
      if (progress < 1) requestAnimationFrame(tick);
      else element.textContent = String(target);
    };
    requestAnimationFrame(tick);
  };
  const observer = new IntersectionObserver((entries) => {
    entries.forEach((entry) => {
      if (!entry.isIntersecting) return;
      animate(entry.target);
      observer.unobserve(entry.target);
    });
  }, { threshold: 0.6 });
  counters.forEach((counter) => observer.observe(counter));
}

function initStageParallax() {
  if (!nodes.stage || !window.matchMedia("(pointer: fine)").matches) return;
  if (window.matchMedia("(prefers-reduced-motion: reduce)").matches) return;
  const update = (event) => {
    const rect = nodes.stage.getBoundingClientRect();
    const x = (event.clientX - rect.left) / rect.width;
    const y = (event.clientY - rect.top) / rect.height;
    const mx = Math.max(-1, Math.min(1, (x - 0.5) * 2));
    const my = Math.max(-1, Math.min(1, (y - 0.5) * 2));
    nodes.stage.style.setProperty("--mx", mx.toFixed(3));
    nodes.stage.style.setProperty("--my", my.toFixed(3));
  };
  nodes.stage.addEventListener("pointermove", update);
  nodes.stage.addEventListener("pointerleave", () => {
    nodes.stage.style.setProperty("--mx", "0");
    nodes.stage.style.setProperty("--my", "0");
  });
}

function initCursorGlow() {
  if (!nodes.glow || !window.matchMedia("(pointer: fine)").matches) return;
  if (window.matchMedia("(prefers-reduced-motion: reduce)").matches) return;
  let lastX = 0;
  let lastY = 0;
  window.addEventListener("pointermove", (event) => {
    lastX = event.clientX;
    lastY = event.clientY;
    if (pointerFrame) return;
    pointerFrame = requestAnimationFrame(() => {
      nodes.glow.style.transform = `translate(${lastX}px, ${lastY}px) translate(-50%, -50%)`;
      nodes.glow.style.opacity = "1";
      pointerFrame = null;
    });
  }, { passive: true });
  document.addEventListener("pointerleave", () => {
    nodes.glow.style.opacity = "0";
  });
}

function initCardGlow() {
  if (!window.matchMedia("(pointer: fine)").matches) return;
  qsa(".bento-card").forEach((card) => {
    card.addEventListener("pointermove", (event) => {
      const rect = card.getBoundingClientRect();
      const x = ((event.clientX - rect.left) / rect.width) * 100;
      const y = ((event.clientY - rect.top) / rect.height) * 100;
      card.style.setProperty("--gx", `${x}%`);
      card.style.setProperty("--gy", `${y}%`);
    });
  });
}

function initCopyCommand() {
  if (!nodes.copyCommand) return;
  nodes.copyCommand.addEventListener("click", async () => {
    const text = nodes.installCommand ? nodes.installCommand.textContent : "";
    if (!text) return;
    try {
      await navigator.clipboard.writeText(text);
    } catch {
      const area = document.createElement("textarea");
      area.value = text;
      area.style.position = "fixed";
      area.style.opacity = "0";
      document.body.append(area);
      area.select();
      document.execCommand("copy");
      area.remove();
    }
    const label = qs("span", nodes.copyCommand);
    if (label) {
      label.textContent = "Copied";
      setTimeout(() => { label.textContent = "Copy"; }, 1800);
    }
    showToast("Command copied", "Paste it into your terminal after the download finishes.");
  });
}

function initToast() {
  if (!nodes.toastClose) return;
  nodes.toastClose.addEventListener("click", () => {
    nodes.toast.classList.remove("show");
    clearTimeout(toastTimer);
  });
}

function initPaletteDemo() {
  const demo = qs("[data-palette-demo]");
  if (!demo) return;
  const query = qs("[data-palette-query]", demo);
  const items = qsa("[data-palette-item]", demo);
  if (!query || items.length < 2) return;

  const reduced = window.matchMedia("(prefers-reduced-motion: reduce)").matches;
  let active = 0;
  let timer = null;

  const setActive = (index) => {
    items.forEach((item, itemIndex) => item.classList.toggle("active", itemIndex === index));
  };

  if (reduced) {
    setActive(0);
    query.textContent = items[0].dataset.query || "";
    return;
  }

  const typeQuery = (text, done) => {
    clearTimeout(timer);
    query.textContent = "";
    let cursor = 0;
    const step = () => {
      cursor += 1;
      query.textContent = text.slice(0, cursor);
      if (cursor < text.length) {
        timer = setTimeout(step, 48 + Math.random() * 42);
      } else if (done) {
        timer = setTimeout(done, 1650);
      }
    };
    step();
  };

  const cycle = () => {
    active = (active + 1) % items.length;
    setActive(active);
    typeQuery(items[active].dataset.query || "", cycle);
  };

  setActive(0);
  typeQuery(items[0].dataset.query || "", () => {
    timer = setTimeout(cycle, 1300);
  });
}

function initMobileDownload() {
  ["#mobileDownload", "#primaryDownload"].forEach((selector) => {
    const link = qs(selector);
    if (!link) return;
    link.addEventListener("click", () => {
      const href = link.getAttribute("href") || "";
      if (!/\.apk(?:$|\?)/i.test(href)) return;
      showToast(
        "Android test APK download started",
        "Transfer the APK to your phone, open it, and allow installs from your browser or file manager. Android 8.0+ required."
      );
    });
  });
}

function initMisc() {
  if (nodes.year) nodes.year.textContent = String(new Date().getFullYear());
  initHeader();
  initNavigation();
  initDownloadMenu();
  initDownloadAnnouncements();
  initReveals();
  initCounters();
  initStageParallax();
  initCursorGlow();
  initCardGlow();
  initCopyCommand();
  initToast();
  initPaletteDemo();
  initMobileDownload();
  fetchStars();
  initDownloads();
}

initMisc();

pub const DEFAULT_BRANCH: &str = "main";
pub const DEFAULT_COMMIT_AUTHOR: &str = "GitSwamp";
pub const DEFAULT_COMMIT_EMAIL: &str = "gitswamp@local";
pub const APP_USER_AGENT: &str = "GitSwamp";

pub const CONFLICT_START: &str = "<<<<<<<";
pub const CONFLICT_MID: &str = "=======";
pub const CONFLICT_END: &str = ">>>>>>>";

#[cfg(windows)]
pub const CREATE_NO_WINDOW: u32 = 0x08000000;

#[cfg(windows)]
pub const COMMON_GIT_PATHS: &[&str] = &[
    r"C:\Program Files\Git\cmd\git.exe",
    r"C:\Program Files\Git\bin\git.exe",
    r"C:\Program Files\Git\mingw64\bin\git.exe",
    r"C:\Program Files (x86)\Git\cmd\git.exe",
    r"C:\Program Files (x86)\Git\bin\git.exe",
];

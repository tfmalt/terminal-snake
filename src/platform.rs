use std::fs;

/// Runtime platform capabilities relevant to this game.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct Platform {
    is_wsl: bool,
}

impl Platform {
    /// Detects platform details from the current runtime environment.
    #[must_use]
    pub fn detect() -> Self {
        Self {
            is_wsl: detect_wsl(),
        }
    }

    /// Returns true when running under Windows Subsystem for Linux.
    #[must_use]
    pub fn is_wsl(self) -> bool {
        self.is_wsl
    }
}

fn detect_wsl() -> bool {
    let Ok(version) = fs::read_to_string("/proc/version") else {
        return false;
    };

    version.to_ascii_lowercase().contains("microsoft")
}

#[cfg(test)]
mod tests {
    use super::Platform;

    #[test]
    fn platform_detection_runs_without_panicking() {
        let _ = Platform::detect();
    }
}

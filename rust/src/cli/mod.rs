use log::LevelFilter;

pub mod database;
pub mod server;

#[derive(Debug, Clone)]
pub struct LogLevelFilter(pub LevelFilter);

impl std::str::FromStr for LogLevelFilter {
    type Err = <LevelFilter as std::str::FromStr>::Err;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        LevelFilter::from_str(s).map(Self)
    }
}

impl Default for LogLevelFilter {
    fn default() -> Self {
        Self(LevelFilter::Info)
    }
}

impl std::fmt::Display for LogLevelFilter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Manifest {
    pub package: Package,
}

#[derive(Debug, Deserialize)]
pub struct Package {
    pub readme: Option<String>,
    pub repository: Option<String>,
}

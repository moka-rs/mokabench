use std::path::{Path, PathBuf};

pub const DATASET_DIR: &str = "./datasets";

#[derive(Clone, Debug)]
pub enum TraceFile {
    S3,
    Ds1,
    Oltp,
}

impl TryFrom<&str> for TraceFile {
    type Error = anyhow::Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value.trim().to_lowercase().as_str() {
            "s3" => Ok(Self::S3),
            "ds1" => Ok(Self::Ds1),
            "oltp" => Ok(Self::Oltp),
            _ => Err(anyhow::anyhow!(r#"Unknown trace file "{}""#, value)),
        }
    }
}

impl TraceFile {
    pub fn path(&self) -> PathBuf {
        let mut p = Path::new(DATASET_DIR).to_path_buf();
        match self {
            Self::S3 => p.push("S3.lis"),
            Self::Ds1 => p.push("DS1.lis"),
            Self::Oltp => p.push("OLTP.lis"),
        }
        p
    }

    pub fn default_capacities(&self) -> &[usize] {
        match self {
            Self::S3 => &[100_000, 400_000, 800_000],
            Self::Ds1 => &[1_000_000, 4_000_000, 8_000_000],
            Self::Oltp => &[250, 1_000, 2_000],
        }
    }
}

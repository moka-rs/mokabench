use std::path::{Path, PathBuf};

pub const DATASET_DIR: &str = "./datasets";

#[derive(Clone, Copy, Debug)]
pub enum TraceFile {
    // From ARC paper: "ARC: A Self-Tuning, Low Overhead Replacement Cache"
    // Traces and paper in the author page: https://researcher.watson.ibm.com/researcher/view_person_subpage.php?id=4700
    S3,
    Ds1,
    Oltp,
    // From LIRS paper: "LIRS: An Efficient Low Inter-reference Recency Set Replacement Policy to Improve Buffer Cache Performance"
    // https://ranger.uta.edu/~sjiang/pubs/papers/jiang02_LIRS.pdf
    Loop,
    Multi1,
    Multi2,
    Multi3,
    TwoPools,
    Sprite,
    // Lirs2
    ZigZag,
}

impl TryFrom<&str> for TraceFile {
    type Error = anyhow::Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value.trim().to_lowercase().as_str() {
            "s3" => Ok(Self::S3),
            "ds1" => Ok(Self::Ds1),
            "oltp" => Ok(Self::Oltp),
            "loop" => Ok(Self::Loop),
            "multi1" => Ok(Self::Multi1),
            "multi2" => Ok(Self::Multi2),
            "multi3" => Ok(Self::Multi3),
            "2_pools" => Ok(Self::ZigZag),
            "sprite" => Ok(Self::Sprite),
            "zigzag" => Ok(Self::ZigZag),
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
            Self::Loop => p.push("loop.trc"),
            Self::Multi1 => p.push("multi1.trc"),
            Self::Multi2 => p.push("multi2.trc"),
            Self::Multi3 => p.push("multi3.trc"),
            Self::TwoPools => p.push("2_pools.trc"),
            Self::Sprite => p.push("sprite.trc"),
            Self::ZigZag => p.push("zigzag.trc"),
        }
        p
    }

    pub fn default_capacities(&self) -> &[usize] {
        match self {
            Self::S3 => &[100_000, 400_000, 800_000],
            Self::Ds1 => &[1_000_000, 4_000_000, 8_000_000],
            Self::Oltp => &[256, 512, 1_000, 2_000],
            Self::Loop => &[256, 512, 768, 1024],
            Self::Multi1 => &[128, 256, 512, 768],
            Self::Multi2 => &[128, 256, 512, 768],
            Self::Multi3 => &[128, 256, 512, 768],
            Self::TwoPools => &[128, 256, 512, 768],
            Self::Sprite => &[128, 256, 512, 768],
            Self::ZigZag => &[128, 256, 512, 768],
        }
    }
}

use std::path::{Path, PathBuf};

pub const DATASET_DIR: &str = "./cache-trace";

#[derive(Clone, Copy, Debug)]
pub enum TraceFileGroup {
    Arc,
    Lirs,
}

impl TraceFileGroup {
    pub fn new(file: TraceFile) -> Self {
        if matches!(
            file,
            TraceFile::Loop
                | TraceFile::Multi1
                | TraceFile::Multi2
                | TraceFile::Multi3
                | TraceFile::TwoPools
                | TraceFile::Sprite
                | TraceFile::ZigZag
        ) {
            Self::Lirs
        } else {
            Self::Arc
        }
    }

    pub fn sub_dir(&self) -> &'static str {
        match self {
            Self::Arc => "arc",
            Self::Lirs => "lirs",
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum TraceFile {
    // From ARC paper: "ARC: A Self-Tuning, Low Overhead Replacement Cache"
    // Traces and paper in the author page: https://researcher.watson.ibm.com/researcher/view_person_subpage.php?id=4700
    ConCat,
    Ds1,
    MergeP,
    MergeS,
    Oltp,
    P1,
    P2,
    P3,
    P4,
    P5,
    P6,
    P7,
    P8,
    P9,
    P10,
    P11,
    P12,
    P13,
    P14,
    S1,
    S2,
    S3,
    Spc1LikeRead,

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
            "concat" => Ok(Self::ConCat),
            "ds1" => Ok(Self::Ds1),
            "merge-p" => Ok(Self::MergeP),
            "merge-s" => Ok(Self::MergeS),
            "oltp" => Ok(Self::Oltp),
            "p1" => Ok(Self::P1),
            "p2" => Ok(Self::P2),
            "p3" => Ok(Self::P3),
            "p4" => Ok(Self::P4),
            "p5" => Ok(Self::P5),
            "p6" => Ok(Self::P6),
            "p7" => Ok(Self::P7),
            "p8" => Ok(Self::P8),
            "p9" => Ok(Self::P9),
            "p10" => Ok(Self::P10),
            "p11" => Ok(Self::P11),
            "p12" => Ok(Self::P12),
            "p13" => Ok(Self::P13),
            "p14" => Ok(Self::P14),
            "s1" => Ok(Self::S1),
            "s2" => Ok(Self::S2),
            "s3" => Ok(Self::S3),
            "spc1likeread" => Ok(Self::Spc1LikeRead),

            "loop" => Ok(Self::Loop),
            "multi1" => Ok(Self::Multi1),
            "multi2" => Ok(Self::Multi2),
            "multi3" => Ok(Self::Multi3),
            "2-pools" => Ok(Self::ZigZag),
            "sprite" => Ok(Self::Sprite),
            "zigzag" => Ok(Self::ZigZag),
            _ => Err(anyhow::anyhow!(r#"Unknown trace file "{}""#, value)),
        }
    }
}

impl TraceFile {
    pub fn path(&self) -> PathBuf {
        let mut p = Path::new(DATASET_DIR).to_path_buf();
        p.push(TraceFileGroup::new(*self).sub_dir());

        match self {
            Self::ConCat => p.push("ConCat.lis"),
            Self::Ds1 => p.push("DS1.lis"),
            Self::MergeP => p.push("MergeP.lis"),
            Self::MergeS => p.push("MergeS.lis"),
            Self::Oltp => p.push("OLTP.lis"),
            Self::P1 => p.push("P1.lis"),
            Self::P2 => p.push("P2.lis"),
            Self::P3 => p.push("P3.lis"),
            Self::P4 => p.push("P4.lis"),
            Self::P5 => p.push("P5.lis"),
            Self::P6 => p.push("P6.lis"),
            Self::P7 => p.push("P7.lis"),
            Self::P8 => p.push("P8.lis"),
            Self::P9 => p.push("P9.lis"),
            Self::P10 => p.push("P10.lis"),
            Self::P11 => p.push("P11.lis"),
            Self::P12 => p.push("P12.lis"),
            Self::P13 => p.push("P13.lis"),
            Self::P14 => p.push("P14.lis"),
            Self::S1 => p.push("S1.lis"),
            Self::S2 => p.push("S2.lis"),
            Self::S3 => p.push("S3.lis"),
            Self::Spc1LikeRead => p.push("spc1likeread.lis"),

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
            Self::ConCat => &[400_000, 3_200_000],
            Self::Ds1 => &[1_000_000, 4_000_000, 8_000_000],
            Self::MergeP => &[400_000, 3_200_000],
            Self::MergeS => &[400_000, 3_200_000],
            Self::Oltp => &[256, 512, 1_000, 2_000],
            Self::P1 => &[20_000, 160_000],
            Self::P2 => &[20_000, 160_000],
            Self::P3 => &[20_000, 160_000],
            Self::P4 => &[20_000, 160_000],
            Self::P5 => &[20_000, 160_000],
            Self::P6 => &[20_000, 160_000],
            Self::P7 => &[20_000, 160_000],
            Self::P8 => &[20_000, 160_000],
            Self::P9 => &[20_000, 160_000],
            Self::P10 => &[20_000, 160_000],
            Self::P11 => &[20_000, 160_000],
            Self::P12 => &[20_000, 160_000],
            Self::P13 => &[20_000, 160_000],
            Self::P14 => &[80_000, 640_000],
            Self::S1 => &[100_000, 800_000],
            Self::S2 => &[100_000, 800_000],
            Self::S3 => &[100_000, 400_000, 800_000],
            Self::Spc1LikeRead => &[500_000, 40_000_000],

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

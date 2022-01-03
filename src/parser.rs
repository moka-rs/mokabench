pub trait TraceParser<T> {
    fn parse(&mut self, line: &str, line_number: usize) -> anyhow::Result<T>;
}

// pub trait TraceEntry {}

#[derive(Debug)]
pub struct ArcTraceEntry {
    range: std::ops::Range<usize>,
    line_number: usize,
}

impl ArcTraceEntry {
    pub fn range(&self) -> std::ops::Range<usize> {
        self.range.clone()
    }

    pub fn line_number(&self) -> usize {
        self.line_number
    }
}

pub struct ArcTraceParser;

impl TraceParser<ArcTraceEntry> for ArcTraceParser {
    fn parse(&mut self, line: &str, line_number: usize) -> anyhow::Result<ArcTraceEntry> {
        let tokens = line.split(|c| c == ' ').take(2).collect::<Vec<_>>();
        if tokens.len() < 2 {
            anyhow::bail!("Wrong number of elements: {}", tokens.len());
        }
        let start = tokens[0].parse::<usize>()?;
        let len = tokens[1].parse::<usize>()?;
        Ok(ArcTraceEntry {
            range: start..(start + len),
            line_number,
        })
    }
}

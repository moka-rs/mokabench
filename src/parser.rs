pub trait TraceParser<T> {
    fn parse(&mut self, line: &str) -> Result<T, Box<dyn std::error::Error>>;
}

// pub trait TraceEntry {}

#[derive(Debug)]
pub struct ArcTraceEntry(pub std::ops::Range<usize>);

// impl TraceEntry for ArcTraceEntry {}

pub struct ArcTraceParser;

impl TraceParser<ArcTraceEntry> for ArcTraceParser {
    fn parse(&mut self, line: &str) -> Result<ArcTraceEntry, Box<dyn std::error::Error>> {
        let tokens = line.split(|c| c == ' ').take(2).collect::<Vec<_>>();
        if tokens.len() < 2 {
            return Err(format!("Wrong number of elements: {}", tokens.len()).into());
        }
        let start = tokens[0].parse::<usize>()?;
        let len = tokens[1].parse::<usize>()?;
        Ok(ArcTraceEntry(start..(start + len)))
    }
}

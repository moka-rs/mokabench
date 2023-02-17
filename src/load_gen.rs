use crate::{
    config::Config,
    parser::{GenericTraceParser, TraceParser},
    Command,
};

pub(crate) fn generate_commands<I>(
    config: &Config,
    max_chunk_size: usize,
    counter: &mut usize,
    chunk: I,
) -> anyhow::Result<Vec<Command>>
where
    I: Iterator<Item = std::io::Result<(usize, String)>>,
{
    let mut parser = GenericTraceParser;
    let mut ops = Vec::with_capacity(max_chunk_size);
    for line_result in chunk {
        let (line_number, line) = line_result?;
        let Some(entry) = parser.parse(&line, line_number)? else { continue };
        *counter += 1;
        if config.invalidate_all && *counter % 100_000 == 0 {
            ops.push(Command::InvalidateAll);
            ops.push(Command::GetOrInsert(entry));
        } else if config.invalidate_entries_if && *counter % 5_000 == 0 {
            ops.push(Command::InvalidateEntriesIf(entry));
        } else if config.size_aware && *counter % 11 == 0 {
            ops.push(Command::Update(entry));
        } else if config.invalidate && *counter % 8 == 0 {
            ops.push(Command::Invalidate(entry));
        } else if config.insert_once && *counter % 3 == 0 {
            ops.push(Command::GetOrInsertOnce(entry));
        } else {
            ops.push(Command::GetOrInsert(entry));
        }

        if config.iterate && *counter % 50_000 == 0 {
            ops.push(Command::Iterate);
        }
    }
    Ok(ops)
}

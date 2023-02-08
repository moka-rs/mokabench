use crate::{config::Config, Command};

pub(crate) fn generate_commands<I>(
    config: &Config,
    max_chunk_size: usize,
    counter: &mut usize,
    chunk: I,
) -> anyhow::Result<Vec<Command>>
where
    I: Iterator<Item = std::io::Result<String>>,
{
    let mut ops = Vec::with_capacity(max_chunk_size);
    for line in chunk {
        let line = line?;
        *counter += 1;
        if config.invalidate_all && *counter % 100_000 == 0 {
            ops.push(Command::InvalidateAll);
            ops.push(Command::GetOrInsert(line, *counter));
        } else if config.invalidate_entries_if && *counter % 5_000 == 0 {
            ops.push(Command::InvalidateEntriesIf(line, *counter));
        } else if config.size_aware && *counter % 11 == 0 {
            ops.push(Command::Update(line, *counter));
        } else if config.invalidate && *counter % 8 == 0 {
            ops.push(Command::Invalidate(line, *counter));
        } else if config.insert_once && *counter % 3 == 0 {
            ops.push(Command::GetOrInsertOnce(line, *counter));
        } else {
            ops.push(Command::GetOrInsert(line, *counter));
        }

        if config.iterate && *counter % 50_000 == 0 {
            ops.push(Command::Iterate);
        }
    }
    Ok(ops)
}

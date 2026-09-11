use rustler::{Encoder, Env, Error, NifResult, Term};
use std::collections::HashMap;
use std::fs::File;

#[rustler::nif]
fn parse_summary_nif(env: Env, path: String) -> NifResult<Term> {
    let file = File::open(&path).map_err(|e| Error::Term(Box::new(format!("{}", e))))?;
    let mut reader = jfrs::reader::JfrReader::new(file);

    let mut chunk_count: u64 = 0;
    let mut event_counts: HashMap<String, u64> = HashMap::new();

    for chunk_result in reader.chunks() {
        let (mut chunk_reader, chunk) =
            chunk_result.map_err(|e| Error::Term(Box::new(format!("{}", e))))?;
        chunk_count += 1;

        for event_result in chunk_reader.events(&chunk) {
            let event = event_result.map_err(|e| Error::Term(Box::new(format!("{}", e))))?;
            let event_name = event.class.name().to_string();
            *event_counts.entry(event_name).or_insert(0) += 1;
        }
    }

    Ok((chunk_count, event_counts).encode(env))
}

rustler::init!("Elixir.Scry.Jfr.Parser");

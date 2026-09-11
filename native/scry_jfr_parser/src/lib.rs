use jfrs::reader::event::Accessor;
use jfrs::reader::type_descriptor::TypeDescriptor;
use jfrs::reader::value_descriptor::{Primitive, ValueDescriptor};
use jfrs::reader::{Chunk, JfrReader};
use rustler::{Encoder, Env, Error, NifResult, Term};
use std::collections::{HashMap, HashSet};
use std::fs::File;

#[rustler::nif]
fn parse_summary_nif(env: Env, path: String) -> NifResult<Term> {
    let file = File::open(&path).map_err(|e| Error::Term(Box::new(format!("{}", e))))?;
    let mut reader = JfrReader::new(file);

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

#[rustler::nif]
fn list_event_types_nif(env: Env, path: String) -> NifResult<Term> {
    let file = File::open(&path).map_err(|e| Error::Term(Box::new(format!("{}", e))))?;
    let mut reader = JfrReader::new(file);

    let mut types: HashSet<String> = HashSet::new();

    for chunk_result in reader.chunks() {
        let (mut chunk_reader, chunk) =
            chunk_result.map_err(|e| Error::Term(Box::new(format!("{}", e))))?;

        for event_result in chunk_reader.events(&chunk) {
            let event = event_result.map_err(|e| Error::Term(Box::new(format!("{}", e))))?;
            types.insert(event.class.name().to_string());
        }
    }

    let mut sorted: Vec<String> = types.into_iter().collect();
    sorted.sort();

    Ok(sorted.encode(env))
}

#[rustler::nif]
fn extract_events_nif<'a>(
    env: Env<'a>,
    path: String,
    event_names: Vec<String>,
) -> NifResult<Term<'a>> {
    let file = File::open(&path).map_err(|e| Error::Term(Box::new(format!("{}", e))))?;
    let mut reader = JfrReader::new(file);

    let filter: HashSet<String> = event_names.into_iter().collect();
    let mut events: Vec<Term<'a>> = Vec::new();

    for chunk_result in reader.chunks() {
        let (mut chunk_reader, chunk) =
            chunk_result.map_err(|e| Error::Term(Box::new(format!("{}", e))))?;

        for event_result in chunk_reader.events(&chunk) {
            let event = event_result.map_err(|e| Error::Term(Box::new(format!("{}", e))))?;
            let event_name = event.class.name().to_string();

            if !filter.is_empty() && !filter.contains(&event_name) {
                continue;
            }

            let accessor = event.value();
            let mut event_map: HashMap<String, Term<'a>> = HashMap::new();
            event_map.insert("event".to_string(), event_name.encode(env));

            if let Some(timestamp) = get_nanoseconds(&accessor, &chunk, "startTime") {
                event_map.insert("timestamp".to_string(), timestamp.encode(env));
            }

            if let Some(duration) = get_nanoseconds(&accessor, &chunk, "duration") {
                event_map.insert("duration".to_string(), duration.encode(env));
            }

            if let Some(thread) = extract_thread_name(&accessor) {
                event_map.insert("thread".to_string(), thread.encode(env));
            }

            if let Some(stack_trace) = extract_stack_trace(env, &accessor) {
                event_map.insert("stack_trace".to_string(), stack_trace);
            }

            let fields = encode_fields(env, &chunk, &accessor, event.class, &["startTime", "duration", "eventThread", "sampledThread", "stackTrace"]);
            event_map.insert("fields".to_string(), fields);

            events.push(event_map.encode(env));
        }
    }

    Ok(events.encode(env))
}

fn get_nanoseconds(accessor: &Accessor, chunk: &Chunk, field_name: &str) -> Option<i64> {
    let ticks = accessor.get_field(field_name)?;
    let ticks_i64 = <i64>::try_from(ticks.value).ok()?;
    Some(ticks_to_nanoseconds(ticks_i64, chunk))
}

fn ticks_to_nanoseconds(ticks: i64, chunk: &Chunk) -> i64 {
    chunk.header.start_time_nanos
        + ((ticks - chunk.header.start_ticks) * 1_000_000_000 / chunk.header.ticks_per_second)
}

fn extract_thread_name(accessor: &Accessor) -> Option<String> {
    let thread = accessor
        .get_field("eventThread")
        .or_else(|| accessor.get_field("sampledThread"))?;

    if let Some(java_name) = thread.get_field("javaName") {
        if let Ok(name) = <&str>::try_from(java_name.value) {
            return Some(name.to_string());
        }
    }

    if let Some(os_name) = thread.get_field("osName") {
        if let Ok(name) = <&str>::try_from(os_name.value) {
            return Some(name.to_string());
        }
    }

    None
}

fn extract_stack_trace<'a, 'b>(
    env: Env<'a>,
    accessor: &Accessor<'b>,
) -> Option<Term<'a>> {
    let stack_trace = accessor.get_field("stackTrace")?;
    let frames_accessor = stack_trace.get_field("frames")?;
    let frames_iter = frames_accessor.as_iter()?;

    let mut frames: Vec<Term<'a>> = Vec::new();

    for frame in frames_iter {
        let mut frame_map: HashMap<String, Term<'a>> = HashMap::new();

        if let Some(method) = frame.get_field("method") {
            let class_name = method
                .get_field("type")
                .and_then(|t| t.get_field("name"))
                .and_then(|n| n.get_field("string"))
                .and_then(|s| <&str>::try_from(s.value).ok())
                .unwrap_or("")
                .encode(env);
            let method_name = method
                .get_field("name")
                .and_then(|n| n.get_field("string"))
                .and_then(|s| <&str>::try_from(s.value).ok())
                .unwrap_or("")
                .encode(env);

            frame_map.insert("class".to_string(), class_name);
            frame_map.insert("method".to_string(), method_name);
        }

        if let Some(line) = frame.get_field("lineNumber") {
            if let Ok(n) = <i32>::try_from(line.value) {
                frame_map.insert("line".to_string(), n.encode(env));
            }
        }

        frames.push(frame_map.encode(env));
    }

    Some(frames.encode(env))
}

fn encode_fields<'a, 'b>(
    env: Env<'a>,
    chunk: &'b Chunk,
    accessor: &Accessor<'b>,
    type_desc: &'b TypeDescriptor,
    exclude: &[&str],
) -> Term<'a> {
    let mut map: HashMap<String, Term<'a>> = HashMap::new();

    for field_desc in type_desc.fields.iter() {
        let name = field_desc.name();
        if exclude.contains(&name) {
            continue;
        }
        if let Some(field_accessor) = accessor.get_field(name) {
            map.insert(name.to_string(), encode_value(env, chunk, field_accessor.value));
        }
    }

    map.encode(env)
}

fn encode_value<'a, 'b>(
    env: Env<'a>,
    chunk: &'b Chunk,
    value: &'b ValueDescriptor,
) -> Term<'a> {
    match value {
        ValueDescriptor::Primitive(p) => encode_primitive(env, p),
        ValueDescriptor::Object(obj) => {
            let mut map: HashMap<String, Term<'a>> = HashMap::new();
            if let Some(type_desc) = chunk.metadata.type_pool.get(obj.class_id) {
                for (idx, field_value) in obj.fields.iter().enumerate() {
                    if let Some(field_desc) = type_desc.fields.get(idx) {
                        map.insert(
                            field_desc.name().to_string(),
                            encode_value(env, chunk, field_value),
                        );
                    }
                }
            }
            map.encode(env)
        }
        ValueDescriptor::Array(arr) => {
            let terms: Vec<Term<'a>> = arr
                .iter()
                .map(|v| encode_value(env, chunk, v))
                .collect();
            terms.encode(env)
        }
        ValueDescriptor::ConstantPool {
            class_id: _,
            constant_index: _,
        } => match Accessor::new(chunk, value).resolve() {
            Some(resolved) => encode_value(env, chunk, resolved.value),
            None => rustler::types::atom::nil().to_term(env),
        },
    }
}

fn encode_primitive<'a>(env: Env<'a>, p: &Primitive) -> Term<'a> {
    match p {
        Primitive::Integer(v) => (*v as i64).encode(env),
        Primitive::Long(v) => v.encode(env),
        Primitive::Float(v) => (*v as f64).encode(env),
        Primitive::Double(v) => v.encode(env),
        Primitive::Character(c) => c.to_string().encode(env),
        Primitive::Boolean(v) => v.encode(env),
        Primitive::Short(v) => (*v as i64).encode(env),
        Primitive::Byte(v) => (*v as i64).encode(env),
        Primitive::NullString => rustler::types::atom::nil().to_term(env),
        Primitive::String(s) => s.encode(env),
    }
}

rustler::init!("Elixir.Scry.Jfr.Parser");

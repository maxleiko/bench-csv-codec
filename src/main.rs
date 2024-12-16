use std::{
    fs::File,
    io::{BufReader, BufWriter, Read, Write},
    os::unix::fs::MetadataExt,
    str::FromStr,
    time::Instant,
};

use anyhow::{Context, Result};
use chrono::{DateTime, Duration, Utc};
use clap::Parser;
use rand::Rng;
use serde::{Deserialize, Serialize};

const BUFFER_CAPACITY: usize = 4 * 1024 * 1024;

#[derive(Debug, Parser)]
struct Args {
    #[arg(
        index = 1,
        default_value = "1000000",
        help = "Number of rows to generate"
    )]
    nb_rows: usize,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let records = create_records(args.nb_rows)?;

    println!(
        "{:>12}{:>12}{:>12}{:>12}{:>12}{:>12}{:>12}",
        "algo", "write_speed", "write_took", "read_speed", "read_took", "filesize", "total_time"
    );
    println!("{}", Benchmark::new("data.csv", Raw).bench(&records)?);
    println!("{}", Benchmark::new("data.csv.gz1", Gzip1).bench(&records)?);
    // println!("{}", Benchmark::new("data.csv.gz9", Gzip9).bench(&records)?);
    println!("{}", Benchmark::new("data.csv.lz4", Lz4).bench(&records)?);
    println!("{}", Benchmark::new("data.csv.sz", Snap).bench(&records)?);
    println!("{}", Benchmark::new("data.csv.zstd", Zstd).bench(&records)?);

    Ok(())
}

#[derive(Debug, Serialize, Deserialize)]
struct Record {
    time: DateTime<Utc>,
    value: i64,
}

fn create_records(nb_rows: usize) -> Result<Vec<Record>> {
    let mut time = DateTime::from_str("2005-01-01T00:00:00Z").context("date")?;
    let mut rng = rand::thread_rng();
    let mut records = Vec::with_capacity(nb_rows);
    for _ in 0..nb_rows {
        records.push(Record {
            time,
            value: rng.gen_range(0..10_000),
        });
        time += Duration::milliseconds(100);
    }
    Ok(records)
}

struct Benchmark<C> {
    filepath: String,
    codec: C,
}

impl<C: Bench> Benchmark<C> {
    fn new(filepath: &str, codec: C) -> Self {
        Self {
            filepath: filepath.into(),
            codec,
        }
    }

    fn bench(&self, records: &[Record]) -> Result<BenchResult> {
        self.codec.bench(&self.filepath, records)
    }
}

struct BenchResult {
    algorithm: &'static str,
    write_duration: std::time::Duration,
    file_size: u64,
    read_duration: std::time::Duration,
}

impl std::fmt::Display for BenchResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let w_speed =
            (self.file_size as f64 / self.write_duration.as_secs() as f64) / 1024.0 / 1024.0;
        let r_speed =
            (self.file_size as f64 / self.read_duration.as_secs() as f64) / 1024.0 / 1024.0;
        write!(
            f,
            "{:>12}{:>8.2}MB/s{:>12.2?}{:>8.2}MB/s{:>12.2?}{:>10.2}MB{:>12.2?}",
            self.algorithm,
            w_speed,
            self.write_duration,
            r_speed,
            self.read_duration,
            self.file_size as f64 / 1024.0 / 1024.0,
            self.write_duration + self.read_duration,
        )
    }
}

trait Bench {
    fn bench(&self, filepath: &str, records: &[Record]) -> Result<BenchResult>;
}

struct Raw;
impl Bench for Raw {
    fn bench(&self, filepath: &str, records: &[Record]) -> Result<BenchResult> {
        // write
        let writer = BufWriter::with_capacity(BUFFER_CAPACITY, File::create(filepath)?);
        let start = Instant::now();
        write_records(writer, records)?;
        let file_size = std::fs::metadata(filepath)?.size();
        let write_duration = start.elapsed();
        // read
        let reader = BufReader::with_capacity(BUFFER_CAPACITY, File::open(filepath)?);
        let start = Instant::now();
        read_records(reader)?;
        let read_duration = start.elapsed();

        Ok(BenchResult {
            algorithm: "raw",
            write_duration,
            file_size,
            read_duration,
        })
    }
}

struct Lz4;
impl Bench for Lz4 {
    fn bench(&self, filepath: &str, records: &[Record]) -> Result<BenchResult> {
        // write
        let writer = lz4::EncoderBuilder::new().build(BufWriter::with_capacity(
            BUFFER_CAPACITY,
            File::create(filepath)?,
        ))?;
        let start = Instant::now();
        write_records(writer, records)?;
        let file_size = std::fs::metadata(filepath)?.size();
        let write_duration = start.elapsed();
        // read
        let reader =
            BufReader::with_capacity(BUFFER_CAPACITY, lz4::Decoder::new(File::open(filepath)?)?);
        let start = Instant::now();
        read_records(reader)?;
        let read_duration = start.elapsed();

        Ok(BenchResult {
            algorithm: "lz4",
            write_duration,
            file_size,
            read_duration,
        })
    }
}

struct Zstd;
impl Bench for Zstd {
    fn bench(&self, filepath: &str, records: &[Record]) -> Result<BenchResult> {
        // write
        let writer = zstd::Encoder::new(File::create(filepath)?, 3)?.auto_finish();
        let start = Instant::now();
        write_records(writer, records)?;
        let file_size = std::fs::metadata(filepath)?.size();
        let write_duration = start.elapsed();
        // read
        let reader =
            BufReader::with_capacity(BUFFER_CAPACITY, zstd::Decoder::new(File::open(filepath)?)?);
        let start = Instant::now();
        read_records(reader)?;
        let read_duration = start.elapsed();

        Ok(BenchResult {
            algorithm: "zstd",
            write_duration,
            file_size,
            read_duration,
        })
    }
}

struct Snap;
impl Bench for Snap {
    fn bench(&self, filepath: &str, records: &[Record]) -> Result<BenchResult> {
        // write
        let writer = snap::write::FrameEncoder::new(BufWriter::with_capacity(
            BUFFER_CAPACITY,
            File::create(filepath)?,
        ));
        let start = Instant::now();
        write_records(writer, records)?;
        let file_size = std::fs::metadata(filepath)?.size();
        let write_duration = start.elapsed();
        // read
        let reader = snap::read::FrameDecoder::new(BufReader::with_capacity(
            BUFFER_CAPACITY,
            File::open(filepath)?,
        ));
        let start = Instant::now();
        read_records(reader)?;
        let read_duration = start.elapsed();

        Ok(BenchResult {
            algorithm: "snap",
            write_duration,
            file_size,
            read_duration,
        })
    }
}

struct Gzip1;
impl Bench for Gzip1 {
    fn bench(&self, filepath: &str, records: &[Record]) -> Result<BenchResult> {
        // write
        let writer = flate2::write::GzEncoder::new(
            BufWriter::with_capacity(BUFFER_CAPACITY, File::create(filepath)?),
            flate2::Compression::new(1),
        );
        let start = Instant::now();
        write_records(writer, records)?;
        let file_size = std::fs::metadata(filepath)?.size();
        let write_duration = start.elapsed();
        // read
        let reader = flate2::read::GzDecoder::new(BufReader::with_capacity(
            BUFFER_CAPACITY,
            File::open(filepath)?,
        ));
        let start = Instant::now();
        read_records(reader)?;
        let read_duration = start.elapsed();

        Ok(BenchResult {
            algorithm: "gzip(1)",
            write_duration,
            file_size,
            read_duration,
        })
    }
}

#[allow(dead_code)]
struct Gzip9;
impl Bench for Gzip9 {
    fn bench(&self, filepath: &str, records: &[Record]) -> Result<BenchResult> {
        // write
        let writer = flate2::write::GzEncoder::new(
            BufWriter::with_capacity(BUFFER_CAPACITY, File::create(filepath)?),
            flate2::Compression::new(9),
        );
        let start = Instant::now();
        write_records(writer, records)?;
        let file_size = std::fs::metadata(filepath)?.size();
        let write_duration = start.elapsed();
        // read
        let reader = flate2::read::GzDecoder::new(BufReader::with_capacity(
            BUFFER_CAPACITY,
            File::open(filepath)?,
        ));
        let start = Instant::now();
        read_records(reader)?;
        let read_duration = start.elapsed();

        Ok(BenchResult {
            algorithm: "gzip(9)",
            write_duration,
            file_size,
            read_duration,
        })
    }
}

fn write_records<W: Write>(writer: W, records: &[Record]) -> Result<()> {
    let mut writer = csv::Writer::from_writer(writer);
    for record in records {
        writer.serialize(record)?;
    }
    writer.flush()?;
    Ok(())
}

fn read_records<R: Read>(reader: R) -> Result<()> {
    let mut reader = csv::Reader::from_reader(reader);
    for result in reader.deserialize() {
        let _record: Record = result?;
    }
    Ok(())
}

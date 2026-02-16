use chrono::NaiveDateTime;
use rusqlite::{Connection, Result, params};

use crate::models::GlucoseReading;

const DATETIME_FMT: &str = "%Y-%m-%dT%H:%M:%S";

pub struct Database {
    conn: Connection,
}

impl Database {
    pub fn open(path: &str) -> Result<Self> {
        let conn = Connection::open(path)?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS readings (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                value REAL NOT NULL,
                recorded_at TEXT NOT NULL,
                created_at TEXT NOT NULL DEFAULT (datetime('now'))
            );",
        )?;
        Ok(Self { conn })
    }

    pub fn insert_reading(&self, value: f64, recorded_at: NaiveDateTime) -> Result<()> {
        self.conn.execute(
            "INSERT INTO readings (value, recorded_at) VALUES (?1, ?2)",
            params![value, recorded_at.format(DATETIME_FMT).to_string()],
        )?;
        Ok(())
    }

    pub fn get_readings(
        &self,
        from: Option<NaiveDateTime>,
        to: Option<NaiveDateTime>,
    ) -> Result<Vec<GlucoseReading>> {
        let (sql, params_vec): (String, Vec<Box<dyn rusqlite::types::ToSql>>) = match (from, to) {
            (Some(f), Some(t)) => (
                "SELECT id, value, recorded_at FROM readings WHERE recorded_at >= ?1 AND recorded_at <= ?2 ORDER BY recorded_at DESC".into(),
                vec![
                    Box::new(f.format(DATETIME_FMT).to_string()) as Box<dyn rusqlite::types::ToSql>,
                    Box::new(t.format(DATETIME_FMT).to_string()),
                ],
            ),
            (Some(f), None) => (
                "SELECT id, value, recorded_at FROM readings WHERE recorded_at >= ?1 ORDER BY recorded_at DESC".into(),
                vec![Box::new(f.format(DATETIME_FMT).to_string()) as Box<dyn rusqlite::types::ToSql>],
            ),
            (None, Some(t)) => (
                "SELECT id, value, recorded_at FROM readings WHERE recorded_at <= ?1 ORDER BY recorded_at DESC".into(),
                vec![Box::new(t.format(DATETIME_FMT).to_string()) as Box<dyn rusqlite::types::ToSql>],
            ),
            (None, None) => (
                "SELECT id, value, recorded_at FROM readings ORDER BY recorded_at DESC".into(),
                vec![],
            ),
        };

        let params_refs: Vec<&dyn rusqlite::types::ToSql> =
            params_vec.iter().map(|p| p.as_ref()).collect();

        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt.query_map(params_refs.as_slice(), |row| {
            let id: i64 = row.get(0)?;
            let value: f64 = row.get(1)?;
            let recorded_at_str: String = row.get(2)?;
            let recorded_at =
                NaiveDateTime::parse_from_str(&recorded_at_str, DATETIME_FMT).unwrap();
            Ok(GlucoseReading {
                id,
                value,
                recorded_at,
            })
        })?;

        rows.collect()
    }

    pub fn delete_reading(&self, id: i64) -> Result<()> {
        self.conn
            .execute("DELETE FROM readings WHERE id = ?1", params![id])?;
        Ok(())
    }
}

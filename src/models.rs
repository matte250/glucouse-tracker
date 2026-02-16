use chrono::NaiveDateTime;

#[derive(Debug, Clone)]
pub struct GlucoseReading {
    pub id: i64,
    pub value: f64,
    pub recorded_at: NaiveDateTime,
}

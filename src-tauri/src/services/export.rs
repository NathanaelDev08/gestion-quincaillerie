use crate::error::AppResult;

pub fn to_csv(headers: &[&str], rows: &[Vec<String>]) -> AppResult<String> {
    let mut w = csv::Writer::from_writer(vec![]);
    w.write_record(headers)
        .map_err(|e| crate::error::AppError::Internal(e.to_string()))?;
    for r in rows {
        w.write_record(r)
            .map_err(|e| crate::error::AppError::Internal(e.to_string()))?;
    }
    let data = w
        .into_inner()
        .map_err(|e| crate::error::AppError::Internal(e.to_string()))?;
    String::from_utf8(data).map_err(|e| crate::error::AppError::Internal(e.to_string()))
}

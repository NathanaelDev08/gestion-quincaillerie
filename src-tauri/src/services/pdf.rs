/// Génération PDF simplifiée : produit un fichier HTML imprimable
/// (le frontend utilise window.print ; le backend écrit le fichier et retourne son chemin)
use crate::error::AppResult;

pub fn build_facture_html(
    company: &serde_json::Value,
    numero: &str,
    client_nom: &str,
    lignes: &[(String, f64, f64, f64)],
    total_ht: f64,
    total_tva: f64,
    total_ttc: f64,
) -> String {
    let rows: String = lignes
        .iter()
        .map(|(d, q, pu, t)| {
            format!("<tr><td>{d}</td><td>{q}</td><td>{pu:.2}</td><td>{t:.2}</td></tr>")
        })
        .collect();
    format!(
        r#"<!DOCTYPE html><html><head><meta charset="utf-8"><style>
        body{{font-family:Arial,sans-serif;margin:40px}}table{{width:100%;border-collapse:collapse}}
        th,td{{border:1px solid #ccc;padding:8px;text-align:left}}h1{{color:#111}}
        </style></head><body>
        <h1>Facture {numero}</h1>
        <p><b>{}</b><br>{}</p>
        <p><b>Client:</b> {client_nom}</p>
        <table><tr><th>Désignation</th><th>Qté</th><th>PU HT</th><th>Total HT</th></tr>{rows}</table>
        <p>Total HT: {total_ht:.2} F CFA<br>TVA: {total_tva:.2} F CFA<br><b>Total TTC: {total_ttc:.2} F CFA</b></p>
        </body></html>"#,
        company.get("company_name").and_then(|v| v.as_str()).unwrap_or("Ma Société"),
        company.get("company_address").and_then(|v| v.as_str()).unwrap_or(""),
    )
}

pub fn write_html_file(dir: &std::path::Path, name: &str, html: &str) -> AppResult<String> {
    std::fs::create_dir_all(dir).map_err(|e| crate::error::AppError::Io(e))?;
    let path = dir.join(name);
    std::fs::write(&path, html).map_err(|e| crate::error::AppError::Io(e))?;
    Ok(path.to_string_lossy().to_string())
}

use anyhow::Result;
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use tracing::info;

pub async fn create_pool(database_url: &str) -> Result<PgPool> {
    let masked_url = mask_database_url(database_url);
    info!("Connecting to database: {}", masked_url);

    let pool = PgPoolOptions::new()
        .max_connections(10)
        .connect(database_url)
        .await?;

    info!("Database connection pool created successfully");

    Ok(pool)
}

fn mask_database_url(url: &str) -> String {
    if url.starts_with("postgres://") || url.starts_with("postgresql://") {
        let without_scheme = url
            .strip_prefix("postgres://")
            .or_else(|| url.strip_prefix("postgresql://"))
            .unwrap_or(url);

        if let Some((user_part, rest)) = without_scheme.split_once('@') {
            if let Some((host_and_port, path_part)) = rest.split_once('/') {
                let host = host_and_port.split(':').next().unwrap_or("localhost");
                let masked = format!("***@{}/*/***", host);
                if !path_part.is_empty() {
                    return format!(
                        "postgres://{}/{}",
                        masked,
                        path_part.split('/').last().unwrap_or("***")
                    );
                }
                return format!("postgres://{}", masked);
            }
            format!("postgres://***@{}", rest.split(':').next().unwrap_or("***"))
        } else {
            format!(
                "postgres://{}/*/***",
                without_scheme.split('/').last().unwrap_or("***")
            )
        }
    } else {
        "***".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::mask_database_url;

    #[test]
    fn mask_database_url_hides_credentials() {
        let url = "postgres://user:pass@localhost:5432/mydb";
        let masked = mask_database_url(url);
        assert_eq!(masked, "postgres://***@localhost/*/***/mydb");
    }

    #[test]
    fn mask_database_url_handles_no_userinfo() {
        let url = "postgres://localhost:5432/mydb";
        let masked = mask_database_url(url);
        assert_eq!(masked, "postgres://mydb/*/***");
    }

    #[test]
    fn mask_database_url_handles_non_postgres() {
        let masked = mask_database_url("sqlite://memory");
        assert_eq!(masked, "***");
    }
}

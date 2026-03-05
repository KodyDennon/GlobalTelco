//! Cloudflare R2 object storage client (S3-compatible).
//!
//! When the `r2` feature is enabled, provides blob storage for world snapshots
//! and cloud saves. Falls back to PostgreSQL BYTEA when R2 is not configured.

#[cfg(feature = "r2")]
use s3::bucket::Bucket;
#[cfg(feature = "r2")]
use s3::creds::Credentials;
#[cfg(feature = "r2")]
use s3::Region;
#[cfg(feature = "r2")]
use tracing::warn;

#[cfg(feature = "r2")]
use uuid::Uuid;

/// R2 storage client wrapping an S3-compatible bucket.
#[cfg(feature = "r2")]
pub struct R2Storage {
    bucket: Box<Bucket>,
}

#[cfg(feature = "r2")]
impl R2Storage {
    /// Create a new R2 storage client.
    ///
    /// `account_id` is the Cloudflare account ID.
    /// `access_key_id` and `secret_key` are the R2 API token credentials.
    /// `bucket_name` is the R2 bucket name.
    pub fn new(
        account_id: &str,
        access_key_id: &str,
        secret_key: &str,
        bucket_name: &str,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let region = Region::Custom {
            region: "auto".to_string(),
            endpoint: format!("https://{account_id}.r2.cloudflarestorage.com"),
        };
        let credentials = Credentials::new(
            Some(access_key_id),
            Some(secret_key),
            None,
            None,
            None,
        )?;
        
        // R2 supports both, but path-style is often more consistent with custom endpoints
        let bucket = Bucket::new(bucket_name, region, credentials)?.with_path_style();

        Ok(Self { bucket })
    }

    /// Upload an object to R2.
    pub async fn put(&self, key: &str, data: &[u8]) -> Result<(), String> {
        match self.bucket.put_object(key, data).await {
            Ok(response) => {
                let code = response.status_code();
                if code == 200 || code == 201 {
                    Ok(())
                } else {
                    let error_text = String::from_utf8_lossy(response.as_slice());
                    if code == 403 {
                        warn!(\"R2 PUT returned 403 Forbidden for key '{key}'. Please check if your R2 API token has 'Edit' permissions for the bucket '{bucket_name}'.\");
                    } else {
                        warn!(\"R2 PUT returned {code} for key '{key}': {error_text}\");
                    }
                    Err(format!(\"R2 PUT returned status {code}: {error_text}\"))
                }
            }
            Err(e) => {
                warn!("R2 PUT transport error for key '{key}': {e}");
                Err(format!("R2 PUT failed: {e}"))
            }
        }
    }

    /// Download an object from R2. Returns None if the key doesn't exist.
    pub async fn get(&self, key: &str) -> Result<Option<Vec<u8>>, String> {
        let response = self
            .bucket
            .get_object(key)
            .await;
        match response {
            Ok(resp) => {
                let code = resp.status_code();
                if code == 200 {
                    Ok(Some(resp.to_vec()))
                } else if code == 404 {
                    Ok(None)
                } else {
                    let error_text = String::from_utf8_lossy(resp.as_slice());
                    warn!("R2 GET returned {code}: {error_text}");
                    Err(format!("R2 GET returned status {code}"))
                }
            }
            Err(e) => {
                let msg = e.to_string();
                if msg.contains("404") || msg.contains("NoSuchKey") {
                    Ok(None)
                } else {
                    warn!("R2 GET transport error: {e}");
                    Err(format!("R2 GET failed: {e}"))
                }
            }
        }
    }

    /// Delete an object from R2.
    pub async fn delete(&self, key: &str) -> Result<(), String> {
        let response = self
            .bucket
            .delete_object(key)
            .await
            .map_err(|e| format!("R2 DELETE failed: {e}"))?;
        let code = response.status_code();
        if code == 200 || code == 204 || code == 404 {
            Ok(())
        } else {
            warn!("R2 DELETE returned {code}");
            Err(format!("R2 DELETE returned status {code}"))
        }
    }

    /// Build the R2 key for a world snapshot.
    pub fn snapshot_key(world_id: Uuid, tick: u64) -> String {
        format!("snapshots/{world_id}/{tick}.bin")
    }

    /// Build the R2 key for a cloud save.
    pub fn save_key(account_id: Uuid, slot: i32) -> String {
        format!("saves/{account_id}/{slot}.bin")
    }
}

/// Stub when R2 feature is not enabled.
#[cfg(not(feature = "r2"))]
pub struct R2Storage;

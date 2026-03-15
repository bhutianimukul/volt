//! Connection manager — saved connections for SSH, databases, Kubernetes.
//! Connections are defined in ~/.config/volt/connections.toml

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionConfig {
    pub connections: HashMap<String, Connection>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Connection {
    #[serde(rename = "ssh")]
    Ssh(SshConnection),
    #[serde(rename = "mysql")]
    Mysql(MysqlConnection),
    #[serde(rename = "postgres")]
    Postgres(PostgresConnection),
    #[serde(rename = "redis")]
    Redis(RedisConnection),
    #[serde(rename = "kubectl")]
    Kubectl(KubectlConnection),
    #[serde(rename = "docker")]
    Docker(DockerConnection),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SshConnection {
    pub host: String,
    pub user: Option<String>,
    pub port: Option<u16>,
    pub identity_file: Option<String>,
    pub proxy_jump: Option<String>,
    #[serde(default)]
    pub forward_agent: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MysqlConnection {
    pub host: String,
    pub port: Option<u16>,
    pub user: String,
    pub database: Option<String>,
    #[serde(default)]
    pub use_keychain: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostgresConnection {
    pub host: String,
    pub port: Option<u16>,
    pub user: String,
    pub database: Option<String>,
    #[serde(default)]
    pub use_keychain: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedisConnection {
    pub host: String,
    pub port: Option<u16>,
    pub database: Option<u8>,
    #[serde(default)]
    pub use_tls: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KubectlConnection {
    pub context: String,
    pub namespace: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DockerConnection {
    pub host: Option<String>,
    pub context: Option<String>,
}

/// Shell-quote a string using single quotes, escaping embedded single quotes.
fn shell_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
}

impl Connection {
    /// Generate the shell command to establish this connection
    pub fn to_command(&self) -> String {
        match self {
            Connection::Ssh(ssh) => {
                let mut cmd = String::from("ssh");
                if let Some(user) = &ssh.user {
                    cmd.push_str(&format!(" {}@{}", shell_quote(user), shell_quote(&ssh.host)));
                } else {
                    cmd.push_str(&format!(" {}", shell_quote(&ssh.host)));
                }
                if let Some(port) = ssh.port {
                    cmd.push_str(&format!(" -p {}", port));
                }
                if let Some(identity) = &ssh.identity_file {
                    cmd.push_str(&format!(" -i {}", shell_quote(identity)));
                }
                if let Some(jump) = &ssh.proxy_jump {
                    cmd.push_str(&format!(" -J {}", shell_quote(jump)));
                }
                if ssh.forward_agent {
                    cmd.push_str(" -A");
                }
                cmd
            }
            Connection::Mysql(mysql) => {
                let mut cmd = format!("mysql -h {} -u {}", shell_quote(&mysql.host), shell_quote(&mysql.user));
                if let Some(port) = mysql.port {
                    cmd.push_str(&format!(" -P {}", port));
                }
                if let Some(db) = &mysql.database {
                    cmd.push_str(&format!(" {}", shell_quote(db)));
                }
                if mysql.use_keychain {
                    cmd.push_str(" --login-path=client");
                } else {
                    cmd.push_str(" -p");
                }
                cmd
            }
            Connection::Postgres(pg) => {
                let mut cmd = format!("psql -h {} -U {}", shell_quote(&pg.host), shell_quote(&pg.user));
                if let Some(port) = pg.port {
                    cmd.push_str(&format!(" -p {}", port));
                }
                if let Some(db) = &pg.database {
                    cmd.push_str(&format!(" -d {}", shell_quote(db)));
                }
                cmd
            }
            Connection::Redis(redis) => {
                let mut cmd = format!("redis-cli -h {}", shell_quote(&redis.host));
                if let Some(port) = redis.port {
                    cmd.push_str(&format!(" -p {}", port));
                }
                if let Some(db) = redis.database {
                    cmd.push_str(&format!(" -n {}", db));
                }
                if redis.use_tls {
                    cmd.push_str(" --tls");
                }
                cmd
            }
            Connection::Kubectl(k8s) => {
                let mut cmd = format!("kubectl --context={}", shell_quote(&k8s.context));
                if let Some(ns) = &k8s.namespace {
                    cmd.push_str(&format!(" -n {}", shell_quote(ns)));
                }
                cmd.push_str(" get pods");
                cmd
            }
            Connection::Docker(docker) => {
                let mut cmd = String::from("docker");
                if let Some(host) = &docker.host {
                    cmd.push_str(&format!(" -H {}", shell_quote(host)));
                }
                if let Some(ctx) = &docker.context {
                    cmd.push_str(&format!(" --context {}", shell_quote(ctx)));
                }
                cmd.push_str(" ps");
                cmd
            }
        }
    }

    /// Get a display name for the connection type
    pub fn type_name(&self) -> &str {
        match self {
            Connection::Ssh(_) => "SSH",
            Connection::Mysql(_) => "MySQL",
            Connection::Postgres(_) => "PostgreSQL",
            Connection::Redis(_) => "Redis",
            Connection::Kubectl(_) => "Kubernetes",
            Connection::Docker(_) => "Docker",
        }
    }

    /// Get an icon for the connection type
    pub fn icon(&self) -> &str {
        match self {
            Connection::Ssh(_) => "🔐",
            Connection::Mysql(_) => "🐬",
            Connection::Postgres(_) => "🐘",
            Connection::Redis(_) => "🔴",
            Connection::Kubectl(_) => "☸",
            Connection::Docker(_) => "🐳",
        }
    }
}

/// Load connections from ~/.config/volt/connections.toml
pub fn load_connections() -> Result<ConnectionConfig, String> {
    let config_path = connection_config_path();
    if !config_path.exists() {
        return Ok(ConnectionConfig {
            connections: HashMap::new(),
        });
    }

    let content = std::fs::read_to_string(&config_path)
        .map_err(|e| format!("Failed to read connections.toml: {}", e))?;
    toml::from_str(&content)
        .map_err(|e| format!("Failed to parse connections.toml: {}", e))
}

/// Get the path to connections.toml
pub fn connection_config_path() -> PathBuf {
    rio_backend::config::config_dir_path().join("connections.toml")
}

/// Search connections by name (fuzzy)
pub fn search_connections<'a>(config: &'a ConnectionConfig, query: &str) -> Vec<(&'a String, &'a Connection)> {
    let q = query.to_lowercase();
    config
        .connections
        .iter()
        .filter(|(name, _)| name.to_lowercase().contains(&q))
        .collect()
}

/// Generate a default connections.toml template
pub fn default_template() -> &'static str {
    r#"# Volt Terminal — Connection Manager
# Define your saved connections here.
# Use `/connection <name>` to connect.

# [connections.prod-server]
# type = "ssh"
# host = "prod.example.com"
# user = "deploy"
# port = 22
# identity_file = "~/.ssh/id_ed25519"

# [connections.staging-db]
# type = "mysql"
# host = "staging-db.example.com"
# user = "app"
# database = "myapp_staging"

# [connections.cache]
# type = "redis"
# host = "redis.example.com"
# port = 6379

# [connections.k8s-prod]
# type = "kubectl"
# context = "production"
# namespace = "default"
"#
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ssh_command() {
        let conn = Connection::Ssh(SshConnection {
            host: "example.com".to_string(),
            user: Some("deploy".to_string()),
            port: Some(2222),
            identity_file: Some("~/.ssh/key".to_string()),
            proxy_jump: None,
            forward_agent: true,
        });
        let cmd = conn.to_command();
        assert!(cmd.contains("ssh 'deploy'@'example.com'"));
        assert!(cmd.contains("-p 2222"));
        assert!(cmd.contains("-i '~/.ssh/key'"));
        assert!(cmd.contains("-A"));
    }

    #[test]
    fn test_mysql_command() {
        let conn = Connection::Mysql(MysqlConnection {
            host: "db.example.com".to_string(),
            port: None,
            user: "root".to_string(),
            database: Some("mydb".to_string()),
            use_keychain: false,
        });
        let cmd = conn.to_command();
        assert!(cmd.contains("mysql -h 'db.example.com' -u 'root'"));
        assert!(cmd.contains("'mydb'"));
    }

    #[test]
    fn test_kubectl_command() {
        let conn = Connection::Kubectl(KubectlConnection {
            context: "production".to_string(),
            namespace: Some("web".to_string()),
        });
        let cmd = conn.to_command();
        assert!(cmd.contains("--context='production'"));
        assert!(cmd.contains("-n 'web'"));
    }

    #[test]
    fn test_shell_quote_injection() {
        // Verify that shell metacharacters are safely wrapped in single quotes
        let conn = Connection::Ssh(SshConnection {
            host: "host$(whoami)".to_string(),
            user: None,
            port: None,
            identity_file: None,
            proxy_jump: None,
            forward_agent: false,
        });
        let cmd = conn.to_command();
        // The host must be single-quoted, preventing command substitution
        assert!(cmd.contains("'host$(whoami)'"));
    }

    #[test]
    fn test_shell_quote_function() {
        assert_eq!(shell_quote("hello"), "'hello'");
        assert_eq!(shell_quote("it's"), "'it'\\''s'");
        assert_eq!(shell_quote("a b"), "'a b'");
    }

    #[test]
    fn test_parse_toml() {
        let toml_str = r#"
[connections.my-server]
type = "ssh"
host = "10.0.0.1"
user = "admin"

[connections.my-redis]
type = "redis"
host = "localhost"
port = 6379
"#;
        let config: ConnectionConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.connections.len(), 2);
        assert!(config.connections.contains_key("my-server"));
        assert!(config.connections.contains_key("my-redis"));
    }

    #[test]
    fn test_search_connections() {
        let toml_str = r#"
[connections.prod-server]
type = "ssh"
host = "prod.example.com"

[connections.staging-db]
type = "mysql"
host = "staging.example.com"
user = "app"
"#;
        let config: ConnectionConfig = toml::from_str(toml_str).unwrap();
        let results = search_connections(&config, "prod");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].0, "prod-server");
    }
}

use crate::db::model::{CmdEntry, DirEntry, SshHostEntry};

/// Home directory prefix: "/Users/alex" on macOS, "/home/alex" on Linux.
fn home() -> &'static str {
    if cfg!(target_os = "macos") {
        "/Users/alex"
    } else {
        "/home/alex"
    }
}

/// Prefix a relative path with the demo home directory.
fn h(rel: &str) -> String {
    format!("{}/{}", home(), rel)
}

/// Generate demo directory entries with realistic paths and varied scores.
pub fn demo_directories(now: i64) -> Vec<DirEntry> {
    let day = 86400;
    let hour = 3600;
    vec![
        dir(&h("projects/webapp"), 42.0, 156, now - 1 * hour, true),
        dir(&h("projects/api-server"), 38.5, 134, now - 2 * hour, true),
        dir(&h("projects/mobile-app"), 28.0, 89, now - 4 * hour, false),
        dir(&h("projects/infra"), 22.3, 67, now - 6 * hour, false),
        dir(&h("projects/data-pipeline"), 18.7, 52, now - 8 * hour, false),
        dir(&h("projects/docs-site"), 15.2, 41, now - 12 * hour, false),
        dir(&h("projects/cli-tool"), 12.8, 35, now - 1 * day, false),
        dir(&h("projects/shared-libs"), 10.4, 28, now - 1 * day, false),
        dir(&h(".config/nvim"), 9.1, 24, now - 2 * day, true),
        dir(&h(".config/alacritty"), 5.3, 12, now - 3 * day, false),
        dir(&h(".config/tmux"), 4.8, 10, now - 4 * day, false),
        dir("/etc/nginx/sites-enabled", 7.6, 18, now - 2 * day, false),
        dir("/var/log/app", 6.2, 15, now - 3 * day, false),
        dir(&h("Documents/specs"), 4.1, 9, now - 5 * day, false),
        dir(&h("Downloads"), 3.5, 8, now - 6 * day, false),
        dir(&h("scripts"), 8.9, 22, now - 1 * day, false),
        dir(&h("projects/webapp/src/components"), 14.6, 38, now - 5 * hour, false),
        dir(&h("projects/api-server/src/handlers"), 11.2, 30, now - 7 * hour, false),
        dir(&h("projects/webapp/tests"), 7.4, 17, now - 2 * day, false),
        dir(&h("projects/api-server/migrations"), 5.8, 13, now - 3 * day, false),
        dir(&h(".ssh"), 3.2, 7, now - 7 * day, false),
        dir("/tmp/debug-session", 2.1, 4, now - 10 * day, false),
        dir(&h("projects/webapp/public"), 6.7, 16, now - 2 * day, false),
        dir(&h("projects/infra/terraform"), 9.5, 25, now - 1 * day, false),
        dir(&h("projects/infra/ansible"), 4.4, 10, now - 5 * day, false),
    ]
}

/// Generate demo command entries with realistic commands and metadata.
pub fn demo_commands(now: i64) -> Vec<CmdEntry> {
    let day = 86400;
    let hour = 3600;
    let webapp = h("projects/webapp");
    let api = h("projects/api-server");
    let infra = h("projects/infra");
    let pipeline = h("projects/data-pipeline");
    let w = webapp.as_str();
    let a = api.as_str();
    let i = infra.as_str();
    let p = pipeline.as_str();
    vec![
        cmd("git status", 45.0, 210, now - 30 * 60, "hook", Some(0), Some(w), Some(120), true),
        cmd("git diff", 32.0, 145, now - 1 * hour, "hook", Some(0), Some(w), Some(250), false),
        cmd("git log --oneline -20", 22.0, 85, now - 2 * hour, "hook", Some(0), Some(a), Some(180), false),
        cmd("git push origin main", 18.5, 62, now - 3 * hour, "hook", Some(0), Some(w), Some(3200), false),
        cmd("git pull --rebase", 16.0, 48, now - 4 * hour, "hook", Some(0), Some(a), Some(4500), false),
        cmd("git stash", 8.2, 24, now - 8 * hour, "hook", Some(0), Some(w), Some(90), false),
        cmd("git checkout -b feature/auth-redesign", 3.1, 6, now - 2 * day, "hook", Some(0), Some(w), Some(150), false),
        cmd("cargo test", 35.0, 178, now - 45 * 60, "hook", Some(0), Some(a), Some(12400), true),
        cmd("cargo build --release", 14.0, 42, now - 5 * hour, "hook", Some(0), Some(a), Some(45000), false),
        cmd("cargo clippy", 10.5, 31, now - 6 * hour, "hook", Some(0), Some(a), Some(8900), false),
        cmd("cargo run -- serve --port 8080", 12.3, 36, now - 3 * hour, "hook", Some(0), Some(a), Some(120000), false),
        cmd("npm run dev", 28.0, 120, now - 1 * hour, "hook", Some(0), Some(w), Some(180000), true),
        cmd("npm run build", 15.0, 45, now - 4 * hour, "hook", Some(0), Some(w), Some(32000), false),
        cmd("npm test -- --watch", 11.0, 33, now - 6 * hour, "hook", Some(0), Some(w), Some(95000), false),
        cmd("npm install", 7.5, 18, now - 1 * day, "hook", Some(0), Some(w), Some(28000), false),
        cmd("docker compose up -d", 20.0, 78, now - 2 * hour, "hook", Some(0), Some(i), Some(15000), false),
        cmd("docker compose logs -f api", 12.0, 35, now - 3 * hour, "hook", Some(0), Some(i), Some(60000), false),
        cmd("docker compose down", 9.8, 28, now - 5 * hour, "hook", Some(0), Some(i), Some(8000), false),
        cmd("docker ps", 14.5, 44, now - 2 * hour, "hook", Some(0), None, Some(350), false),
        cmd("kubectl get pods -n production", 16.5, 50, now - 3 * hour, "hook", Some(0), Some(i), Some(1200), false),
        cmd("kubectl logs -f deploy/api --tail=100", 8.0, 22, now - 6 * hour, "hook", Some(0), Some(i), Some(30000), false),
        cmd("kubectl apply -f k8s/", 5.5, 14, now - 1 * day, "hook", Some(0), Some(i), Some(5600), false),
        cmd("ssh prod-web-01", 11.5, 34, now - 4 * hour, "hook", Some(0), None, Some(180000), false),
        cmd("ssh staging-db", 6.8, 16, now - 8 * hour, "hook", Some(0), None, Some(120000), false),
        cmd("python scripts/migrate.py --dry-run", 7.2, 19, now - 1 * day, "hook", Some(0), Some(p), Some(45000), false),
        cmd("python scripts/backfill.py --since 2026-03-01", 3.8, 8, now - 3 * day, "hook", Some(0), Some(p), Some(320000), false),
        cmd("make deploy-staging", 9.0, 26, now - 5 * hour, "hook", Some(0), Some(i), Some(95000), false),
        cmd("make lint", 13.0, 39, now - 2 * hour, "hook", Some(0), Some(w), Some(6700), false),
        cmd("curl -s localhost:8080/health | jq .", 8.5, 23, now - 4 * hour, "hook", Some(0), Some(a), Some(280), false),
        cmd("vim ~/.config/nvim/init.lua", 6.0, 15, now - 1 * day, "hook", Some(0), None, Some(600000), false),
        cmd("htop", 5.0, 12, now - 2 * day, "hook", Some(0), None, Some(45000), false),
        cmd("tail -f /var/log/app/error.log", 4.5, 11, now - 2 * day, "hook", Some(0), None, Some(120000), false),
        cmd("pg_dump -Fc production > backup.dump", 3.0, 6, now - 5 * day, "hook", Some(0), Some(i), Some(180000), false),
        cmd("psql -d production -c 'SELECT count(*) FROM users'", 5.2, 13, now - 1 * day, "hook", Some(0), Some(p), Some(450), false),
        cmd("terraform plan", 7.8, 20, now - 8 * hour, "hook", Some(0), Some(i), Some(25000), false),
        cmd("terraform apply -auto-approve", 4.0, 9, now - 2 * day, "hook", Some(0), Some(i), Some(180000), false),
        cmd("ansible-playbook deploy.yml -l staging", 3.5, 7, now - 3 * day, "hook", Some(0), Some(i), Some(240000), false),
        cmd("grep -rn 'TODO' src/", 6.5, 16, now - 6 * hour, "hook", Some(0), Some(w), Some(800), false),
        cmd("find . -name '*.log' -mtime +7 -delete", 2.0, 4, now - 7 * day, "hook", Some(0), None, Some(1200), false),
        cmd("cat /etc/hosts", 1.8, 3, now - 10 * day, "history", Some(0), None, Some(50), false),
    ]
}

/// Generate demo SSH host entries.
pub fn demo_ssh_hosts(now: i64) -> Vec<SshHostEntry> {
    let day = 86400;
    let hour = 3600;
    vec![
        ssh("prod-web-01", Some("10.0.1.10"), None, Some("deploy"), 18.0, 52, now - 4 * hour, true, "config"),
        ssh("prod-web-02", Some("10.0.1.11"), None, Some("deploy"), 12.0, 34, now - 6 * hour, false, "config"),
        ssh("prod-db", Some("10.0.2.10"), Some(5432), Some("postgres"), 8.5, 22, now - 8 * hour, true, "config"),
        ssh("staging-web", Some("10.1.1.10"), None, Some("deploy"), 14.0, 40, now - 3 * hour, false, "config"),
        ssh("staging-db", Some("10.1.2.10"), Some(5432), Some("postgres"), 6.8, 16, now - 1 * day, false, "config"),
        ssh("ci-runner", Some("10.2.0.5"), None, Some("ci"), 9.2, 25, now - 5 * hour, false, "config"),
        ssh("bastion", Some("bastion.example.com"), Some(2222), Some("alex"), 15.0, 45, now - 2 * hour, true, "config"),
        ssh("dev-gpu", Some("gpu.internal.example.com"), None, Some("alex"), 5.5, 13, now - 2 * day, false, "config"),
        ssh("monitoring", Some("10.3.0.10"), None, Some("ops"), 4.2, 10, now - 3 * day, false, "config"),
        ssh("backup-server", Some("10.4.0.5"), None, Some("backup"), 2.8, 6, now - 5 * day, false, "config"),
    ]
}

fn dir(path: &str, score: f64, visit_count: i64, last_visit: i64, is_favorite: bool) -> DirEntry {
    DirEntry {
        id: 0,
        path: path.to_string(),
        score,
        visit_count,
        last_visit,
        is_favorite,
    }
}

fn cmd(
    command: &str,
    score: f64,
    use_count: i64,
    last_used: i64,
    source: &str,
    exit_code: Option<i64>,
    cwd: Option<&str>,
    duration_ms: Option<i64>,
    is_favorite: bool,
) -> CmdEntry {
    CmdEntry {
        id: 0,
        command: command.to_string(),
        score,
        use_count,
        last_used,
        is_favorite,
        source: source.to_string(),
        exit_code,
        cwd: cwd.map(|s| s.to_string()),
        duration_ms,
    }
}

fn ssh(
    host: &str,
    hostname: Option<&str>,
    port: Option<i32>,
    user: Option<&str>,
    score: f64,
    use_count: i64,
    last_used: i64,
    is_favorite: bool,
    source: &str,
) -> SshHostEntry {
    SshHostEntry {
        id: 0,
        host: host.to_string(),
        hostname: hostname.map(|s| s.to_string()),
        port,
        user: user.map(|s| s.to_string()),
        score,
        use_count,
        last_used,
        is_favorite,
        source: source.to_string(),
    }
}

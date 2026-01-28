# systemd (VPS)

This is an example `systemd` unit to run Luciuz as a service.

Save as `/etc/systemd/system/luciuz.service`:

```ini
[Unit]
Description=Luciuz Web Server
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
User=zentra
Group=zentra
WorkingDirectory=/home/zentra/LuciuzWeb
Environment=RUST_LOG=info
ExecStart=/home/zentra/LuciuzWeb/target/release/luciuz run -c /home/zentra/LuciuzWeb/luciuz.toml

AmbientCapabilities=CAP_NET_BIND_SERVICE
CapabilityBoundingSet=CAP_NET_BIND_SERVICE

Restart=always
RestartSec=2

[Install]
WantedBy=multi-user.target
```

Reload and start:

```bash
sudo systemctl daemon-reload
sudo systemctl enable --now luciuz
sudo systemctl status luciuz --no-pager
sudo journalctl -u luciuz -f
```

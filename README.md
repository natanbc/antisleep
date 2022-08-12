# Antisleep

Small Windows service that can disable and re-enable sleep via HTTP.

# Installation

```powershell
cargo build --release

mkdir -Force $env:AppData\antisleep

$path = "$env:AppData\antisleep\antisleep.exe"
cp target/release/antisleep.exe $path
cp config.toml.example $env:AppData\antisleep\config.toml

Set-ItemProperty -Path HKCU:SOFTWARE\Microsoft\Windows\CurrentVersion\Run -Name AntiSleep -Value $path
```

# API

```
POST /keep-awake?name=<name>&password=<password>
```
Disables sleep and returns an ID for this wake request in the response body.
The ID is used to inform the server that whatever task needed sleep to be suspended
is done and sleep can now be turned back on. Once all tasks are done, sleep is
re-enabled. The name does not need to be unique, and is used purely for informational
purposes.

If enabled on the configuration, the same password must be provided.

```
POST /task-done?id=<id>&password=<password>
```
Marks a task as done. The ID must be one previously returned by `POST /keep-awake`.

If enabled on the configuration, the same password must be provided.

```
GET /wakers
```
Returns a comma-separated list of tasks currently causing sleep to be disabled.

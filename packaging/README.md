# Packaging notes

## The polkit policy and AppImage do not compose

`polkit/nz.linkr.luxor-optimizer.policy` gives the privileged helper a real
authorization prompt — one that names the change instead of saying
"authenticate to run /some/path as root".

It only takes effect for a **system-installed** helper. A polkit action must
annotate `org.freedesktop.policykit.exec.path` with a stable absolute path, and
an AppImage runs from a FUSE mount under `/tmp/.mount_XXXXXX` whose path differs
on every launch. There is no path that can be written into the policy ahead of
time, and pointing it at a user-writable location would be worse than the
generic prompt: anyone who could write there would get a root-exec primitive.

What this means in practice:

| Distribution | Helper location | Prompt |
|---|---|---|
| `.deb` / `.rpm` | `/usr/libexec/luxor-optimizer/luxor-helper` | Descriptive, from this policy |
| AppImage | inside the mount | Generic pkexec fallback |
| `scripts/install.sh` (user-local) | `~/.local/opt/luxor` | Generic pkexec fallback |

The README currently calls AppImage the primary artifact. That trades a
meaningful consent prompt for portability. Worth revisiting: the deb/rpm targets
are already configured in `tauri.conf.json`, and a privileged desktop tool is a
reasonable thing to ask people to install properly.

`scripts/install.sh` deliberately does **not** install the policy file. Doing so
needs root and writes to `/usr/share/polkit-1/actions/`, which is not something
a user-local installer should silently do.

## Installing the policy manually

For a system install, place the file and ensure the helper sits at the annotated
path:

```bash
sudo install -Dm644 packaging/polkit/nz.linkr.luxor-optimizer.policy \
  /usr/share/polkit-1/actions/nz.linkr.luxor-optimizer.policy
sudo install -Dm755 src-tauri/helper/target/release/luxor-helper \
  /usr/libexec/luxor-optimizer/luxor-helper
```

polkit picks up new action files without a restart.

## Helper ownership

The helper must be owned by root and not writable by anyone else, or the
authorization it gains can be redirected:

```bash
sudo chown root:root /usr/libexec/luxor-optimizer/luxor-helper
sudo chmod 755 /usr/libexec/luxor-optimizer/luxor-helper
```

It is intentionally **not** setuid. Elevation comes from pkexec, which means
every invocation is subject to the policy above and is logged by polkit.

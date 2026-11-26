# Compatibility patch

Upstream xattr 0.2.3 (MIT OR Apache-2.0), pulled by GPUI 0.2.2 → gpui_http_client → zed-async-tar. Original licenses retained. Only src/sys/mod.rs is changed: Linux/Android use ENODATA instead of the removed libc ENOATTR alias; BSD/macOS retain ENOATTR. This is the missing-attribute error used by xattr get. No application behavior is moved into the vendor crate. Revisit/remove when upgrading GPUI. Downgrading libc alone is incompatible with mio 1.2.5 requiring libc ^0.2.190. No Git dependency or global registry files are modified.

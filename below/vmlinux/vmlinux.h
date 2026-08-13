/* Stable include name for below's BPF programs (open-source/cargo build). The
 * vendored kernel BTF is the version-pinned vmlinux_601.h; include it here so
 * the .bpf.c files can include a stable <vmlinux.h>. */
#include "vmlinux_601.h"

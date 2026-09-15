#!/bin/bash
# Root-only watchdog smoke test.

set -euo pipefail

RUN_ID="$$-$(date +%s%N)"
SCOPE="below-wd-test-$$"
CG="/sys/fs/cgroup/system.slice/${SCOPE}.scope"
RECORD_CG="$CG/record-thread"
STORE_WRITER_CG="$CG/store-writer-thread"
WATCHDOG_TIMEOUT=3
RECORD_FREEZE_TIMEOUT_SECS=10
STORE_WRITER_FREEZE_TIMEOUT_SECS=75
CONTROL_SECS=5
RECOVERY_SILENCE_SECS=$((WATCHDOG_TIMEOUT + 2))

BELOW_BIN=""
BELOW_PID=""
WORKDIR=""

log() { echo "[$(date +%H:%M:%S)] $*"; }

usage() {
    echo "Usage: sudo $0 --below-binary PATH" >&2
    exit 1
}

cleanup() {
    set +e
    [[ -w "$RECORD_CG/cgroup.freeze" ]] && echo 0 >"$RECORD_CG/cgroup.freeze"
    [[ -w "$STORE_WRITER_CG/cgroup.freeze" ]] && echo 0 >"$STORE_WRITER_CG/cgroup.freeze"
    systemctl stop "${SCOPE}.scope" 2>/dev/null
    if [[ -n "$WORKDIR" && "$WORKDIR" == /tmp/below-watchdog.* ]]; then
        rm -rf "$WORKDIR"
    fi
}

mark() { echo "<4>below-wd-test-marker $RUN_ID-$1" >/dev/kmsg; }
since_mark() { dmesg | sed -n "/below-wd-test-marker $RUN_ID-$1/,\$p"; }
reports_since() {
    since_mark "$1" |
        grep "below watchdog: kind=stall" |
        grep " pid=$BELOW_PID " || true
}

fail_if_below_exited() {
    if [[ -n "$BELOW_PID" ]] && ! kill -0 "$BELOW_PID" 2>/dev/null; then
        echo "below exited unexpectedly; output follows:" >&2
        cat "$WORKDIR/below.out" >&2
        exit 1
    fi
}

wait_while_alive() {
    local seconds="$1"
    local tick
    for ((tick = 0; tick < seconds * 10; tick++)); do
        fail_if_below_exited
        sleep 0.1
    done
    fail_if_below_exited
}

wait_for_frozen() {
    local cgroup="$1"
    local tick
    for ((tick = 0; tick < 100; tick++)); do
        fail_if_below_exited
        if awk '$1 == "frozen" && $2 == "1" { found = 1 } END { exit !found }' \
            "$cgroup/cgroup.events"; then
            return 0
        fi
        sleep 0.1
    done
    echo "$cgroup did not freeze" >&2
    exit 1
}

wait_for_reports() {
    local phase="$1"
    local required="$2"
    local timeout="$3"
    local reports tick
    for ((tick = 0; tick < timeout; tick++)); do
        fail_if_below_exited
        reports=$(reports_since "$phase")
        if [[ "$(printf '%s\n' "$reports" | awk 'NF { count++ } END { print count + 0 }')" -ge "$required" ]]; then
            return 0
        fi
        sleep 1
    done
    return 1
}

stored_sample_exists() {
    local data_file
    for data_file in "$WORKDIR"/store/data_*; do
        [[ -s "$data_file" ]] && return 0
    done
    return 1
}

thread_tid() {
    local wanted="$1"
    local task comm
    for task in "/proc/$BELOW_PID"/task/*; do
        if read -r comm <"$task/comm" && [[ "$comm" == "$wanted" ]]; then
            basename "$task"
            return 0
        fi
    done
    return 1
}

validate_reports() {
    local reports="$1"
    printf '%s\n' "$reports" | awk \
        -v record_tid="$BELOW_PID" \
        -v store_writer_tid="$STORE_WRITER_TID" \
        -v timeout_ms="$((WATCHDOG_TIMEOUT * 1000))" '
        function value(key, i, field) {
            for (i = 1; i <= NF; i++) {
                split($i, field, "=")
                if (field[1] == key) {
                    return field[2]
                }
            }
            return ""
        }
        function is_uint(value) {
            return value ~ /^[0-9]+$/
        }
        {
            age = value("heartbeat_age_ms")
            heartbeat_value = value("heartbeat_mono_ms")
            reported_timeout = value("timeout_ms")
            record_truncated = value("record_stack_truncated")
            store_writer_truncated = value("store_writer_stack_truncated")
            if (!is_uint(age) || !is_uint(reported_timeout) ||
                age + 0 < timeout_ms ||
                !is_uint(heartbeat_value) ||
                value("record_stack_status") != "ok" ||
                value("store_writer_stack_status") != "ok" ||
                value("record_stack") == "" ||
                value("store_writer_stack") == "" ||
                record_truncated !~ /^[01]$/ ||
                store_writer_truncated !~ /^[01]$/ ||
                reported_timeout + 0 != timeout_ms ||
                value("record_tid") != record_tid ||
                value("store_writer_tid") != store_writer_tid ||
                record_tid == store_writer_tid) {
                exit 1
            }
            if (count == 0) {
                heartbeat = heartbeat_value
            } else {
                delta = age - heartbeat_age
                if (heartbeat_value != heartbeat ||
                    delta < timeout_ms - timeout_ms / 2 ||
                    delta > timeout_ms + timeout_ms / 2) {
                    exit 1
                }
            }
            heartbeat_age = age
            count++
        }
        END {
            if (count < 2) {
                exit 1
            }
            print heartbeat
        }
    '
}

assert_silent_after_recovery() {
    local phase="$1"
    mark "$phase-recovered"
    wait_while_alive "$RECOVERY_SILENCE_SECS"
    [[ -z "$(reports_since "$phase-recovered")" ]] || {
        echo "$phase continued reporting after recovery" >&2
        reports_since "$phase-recovered" >&2
        exit 1
    }
}

check_phase() {
    local phase="$1"
    local reports="$2"
    local heartbeat
    [[ -n "$reports" ]] || {
        echo "$phase produced no watchdog reports" >&2
        exit 1
    }
    while IFS= read -r report; do
        echo "    $report"
    done <<<"$reports"

    if ! heartbeat=$(validate_reports "$reports"); then
        echo "$phase reports did not contain increasing combined samples" >&2
        exit 1
    fi
    log "$phase reported repeated combined samples for heartbeat $heartbeat"
}

while [[ $# -gt 0 ]]; do
    case "$1" in
        --below-binary)
            BELOW_BIN="$2"
            shift 2
            ;;
        *) usage ;;
    esac
done

[[ "$(id -u)" == "0" ]] || {
    echo "must run as root to use cgroups and /dev/kmsg" >&2
    exit 1
}
[[ -x "$BELOW_BIN" ]] || usage
BELOW_BIN=$(readlink -f "$BELOW_BIN")

WORKDIR=$(mktemp -d /tmp/below-watchdog.XXXXXX)
trap cleanup EXIT
mkdir -p "$WORKDIR/log" "$WORKDIR/store"
cat >"$WORKDIR/below.conf" <<EOF
log_dir = "$WORKDIR/log"
store_dir = "$WORKDIR/store"
EOF

log "starting below in ${SCOPE}.scope"
systemd-run --scope --quiet --unit="$SCOPE" --slice=system.slice \
    "$BELOW_BIN" --config "$WORKDIR/below.conf" record \
    --interval-s 1 \
    --port 0 \
    --disable-exitstats \
    --disable-disk-stat \
    --watchdog-timeout-s "$WATCHDOG_TIMEOUT" \
    >"$WORKDIR/below.out" 2>&1 &

for _ in $(seq 1 100); do
    if [[ -d "$CG" ]]; then
        while read -r pid; do
            if [[ "$(readlink -f "/proc/$pid/exe" 2>/dev/null || true)" == "$BELOW_BIN" ]]; then
                BELOW_PID="$pid"
                break 2
            fi
        done <"$CG/cgroup.procs"
    fi
    sleep 0.1
done
[[ -n "$BELOW_PID" ]] || {
    echo "below did not start; output follows:" >&2
    cat "$WORKDIR/below.out" >&2
    exit 1
}

log "waiting for the first stored sample"
for _ in $(seq 1 300); do
    fail_if_below_exited
    stored_sample_exists && break
    sleep 0.1
done
stored_sample_exists || {
    echo "below did not store a sample; output follows:" >&2
    cat "$WORKDIR/below.out" >&2
    exit 1
}

log "checking ${CONTROL_SECS}s of healthy operation"
mark control
wait_while_alive "$CONTROL_SECS"
[[ -z "$(reports_since control)" ]] || {
    echo "healthy record loop produced a watchdog report" >&2
    reports_since control >&2
    exit 1
}

mkdir "$RECORD_CG"
echo threaded >"$RECORD_CG/cgroup.type"
echo "$BELOW_PID" >"$RECORD_CG/cgroup.threads"
STORE_WRITER_TID=$(thread_tid store_writer) || {
    echo "could not find the store_writer thread" >&2
    exit 1
}

log "freezing record thread until two watchdog samples arrive"
mark record-freeze
echo 1 >"$RECORD_CG/cgroup.freeze"
wait_for_frozen "$RECORD_CG"
if ! wait_for_reports record-freeze 2 "$RECORD_FREEZE_TIMEOUT_SECS"; then
    echo "record-thread freeze produced fewer than two reports" >&2
    exit 1
fi
echo 0 >"$RECORD_CG/cgroup.freeze"
wait_while_alive 2
check_phase "record-thread freeze" "$(reports_since record-freeze)"
assert_silent_after_recovery record-thread

mkdir "$STORE_WRITER_CG"
echo threaded >"$STORE_WRITER_CG/cgroup.type"
echo "$STORE_WRITER_TID" >"$STORE_WRITER_CG/cgroup.threads"

log "freezing store_writer until the ten-entry queue blocks the record thread"
mark store-writer-freeze
echo 1 >"$STORE_WRITER_CG/cgroup.freeze"
wait_for_frozen "$STORE_WRITER_CG"
if ! wait_for_reports store-writer-freeze 2 "$STORE_WRITER_FREEZE_TIMEOUT_SECS"; then
    echo "store-writer freeze produced fewer than two reports in ${STORE_WRITER_FREEZE_TIMEOUT_SECS}s" >&2
    exit 1
fi
echo 0 >"$STORE_WRITER_CG/cgroup.freeze"
wait_while_alive 2
check_phase "store-writer freeze" "$(reports_since store-writer-freeze)"
assert_silent_after_recovery store-writer

log "PASS: watchdog reported repeated combined samples for both freeze modes"

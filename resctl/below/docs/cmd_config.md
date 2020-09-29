# Below commands

## How to run command
* Invoke command palette by typing `:`, you should notice the command palette on the bottom becomes highlighted.
* Input command and arguments separated by space:
```
sort cpu_usage
```
* Hit `<Enter>` to submit

## Find help
Press `?` or input `help` in the command palette should bring you the help menu.

You will find two boxes in the help menu. The upper box shows some reserved hot keys with its function and the bottom box
shows the current supported commands with 5 columns:
* Command: The string command that you need to type in command palette. (Not customizable).
* Short Cmd: The short version of "Command". (Not customizable).
* Hot Key: Invoke such command without entering the command palette. If such command requires an argument, a popup box will be showed.
* Args: Supported argument(s)
* Description: man for the command.

## Customize hot key
By default most commands will have a hot key associated with it. A user can setup their own hot key map in file `$HOME/.config/below/cmdrc`. The map should be in form of `COMMAND = HOTKEY`. Here's an example of a working `cmdrc` config:
```
next_tab = 'b'
prev_tab = 'ctrl-c'
next_col = 'ctrlshift-tab'
```
Here's a list of supported hot keys:
```
{char}
ctrl-{char}
alt-{char}
ctrl-{KEY}
alt-{KEY}
shift-{KEY}
altshift-{KEY}
ctrlshift-{KEY}
ctrlalt-{KEY}

// KEY can be one of the followings
tab
enter
backspace
left
right
up
down
ins
del
home
end
page_up
page_down
esc
```

## Supported sort arguments
### Cgroup
```
cpu_usage, cpu_user, cpu_sys, nr_periods, nr_throttled, throttled

mem_total, swap, anon, file, kernel_stack, slab, sock, shmem, file_mapped, file_dirty,
file_writeback, anon_thp, inactive_anon, active_anon, inactive_file, active_file,
unevictable, slab_reclaimable, slab_unreclaimable, pgfault, pgmajfault, workingset_refault,
workingset_activate, workingset_node_reclaim, pgrefill, pgscan, pgsteal, pgactivate,
pgdeactivate, pglazyfree, pglazyfreed, thp_fault_alloc, thp_collapse_alloc


cpu_some, mem_some, mem_full, io_some, io_full


read_bps, write_bps, read_iops, write_iops, discard_bps, discard_iops, rw_total
```

### Process
```
pid, ppid, comm, state, uptime, cgroup, cmdline

cpu_user, cpu_sys, cpu_num_threads, cpu_total

rss, vm_size, lock, pin, anon, file, shmem, pte, swap, huage_tlb, minor_faults, major_faults

read, write, io_total
```

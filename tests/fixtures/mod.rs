// Test fixtures module
// Sample proc/sys file contents for testing

pub const SAMPLE_LOADAVG: &str = "0.01 0.05 0.10 1/234 5678";

pub const SAMPLE_UPTIME: &str = "12345.67 98765.43";

pub const SAMPLE_MEMINFO: &str = r#"MemTotal:       16336236 kB
MemFree:         1234567 kB
MemAvailable:    8765432 kB
Buffers:          123456 kB
Cached:          2345678 kB
SwapCached:            0 kB
Active:          4567890 kB
Inactive:        3456789 kB
"#;

pub const SAMPLE_STAT: &str = r#"cpu  1234567 12345 234567 12345678 45678 12345 6789 0 0 0
cpu0 123456 1234 23456 1234567 4567 1234 678 0 0 0
cpu1 123456 1234 23456 1234567 4567 1234 678 0 0 0
intr 123456789 12 345 678 0 0 0 0 0 0 0 0 0 0 0 0 0
ctxt 12345678901
btime 1234567890
processes 123456
procs_running 3
procs_blocked 0
softirq 12345678 1234 567890 0 1234 5678 0 90123 456789 0 1234567
"#;

pub const SAMPLE_VMSTAT: &str = r#"pgpgin 12345678
pgpgout 87654321
pswpin 0
pswpout 100
pgfault 123456789
pgmajfault 1234
thp_fault_alloc 5678
thp_collapse_alloc 123
"#;

pub const SAMPLE_BUDDYINFO: &str = r#"Node 0, zone      DMA      1      1      1      0      2      1      1      0      1      1      3
Node 0, zone    DMA32     12     34     56      78     90     12      3      4      5      6      7
Node 0, zone   Normal    123    456    789    1234   5678   9012   3456   7890   1234   5678   9012
"#;

pub const SAMPLE_SLABINFO: &str = r#"slabinfo - version: 2.1
# name            <active_objs> <num_objs> <objsize> <objperslab> <pagesperslab> : tunables <limit> <batchcount> <sharedfactor> : slabdata <active_slabs> <num_slabs> <sharedavail>
kmalloc-8192         100    100   8192    4    8 : tunables    0    0    0 : slabdata     25     25      0
kmalloc-4096         200    200   4096    8    8 : tunables    0    0    0 : slabdata     25     25      0
kmalloc-2048         400    400   2048   16    8 : tunables    0    0    0 : slabdata     25     25      0
"#;

pub const SAMPLE_SOCKSTAT: &str = r#"sockets: used 1234
TCP: inuse 45 orphan 0 tw 67 alloc 89 mem 123
UDP: inuse 10 mem 5
UDPLITE: inuse 0
RAW: inuse 0
FRAG: inuse 0 memory 0
"#;

pub const SAMPLE_NETSTAT: &str = r#"TcpExt: SyncookiesSent SyncookiesRecv SyncookiesFailed EmbryonicRsts PruneCalled RcvPruned OfoPruned OutOfWindowIcmps LockDroppedIcmps
TcpExt: 0 0 100 0 0 0 0 0 0
IpExt: InNoRoutes InTruncatedPkts InMcastPkts OutMcastPkts InBcastPkts OutBcastPkts
IpExt: 0 0 12345 6789 98765 43210
"#;

pub const SAMPLE_SCHEDSTAT: &str = r#"version 15
timestamp 1234567890
cpu0 0 0 1234567890 0 0 0 890123456789 7890123456789
cpu1 0 0 2345678901 0 0 0 901234567890 8901234567890
"#;

pub const SAMPLE_CPU_STAT_V2: &str = r#"usage_usec 12345678
user_usec 6789012
system_usec 5456789
nr_periods 0
nr_throttled 0
throttled_usec 0
"#;

pub const SAMPLE_IO_STAT_V2: &str = r#"8:0 rbytes=12345 wbytes=67890 rios=100 wios=200 dbytes=0 dios=0
8:16 rbytes=54321 wbytes=9876 rios=50 wios=100 dbytes=0 dios=0
"#;

pub const SAMPLE_ZONEINFO: &str = r#"Node 0, zone      DMA
  pages free     3456
        min      12
        low      15
        high     18
        spanned  4095
        present  3997
        managed  3840
        nr_free_pages 3456
        nr_zone_inactive_anon 0
        nr_zone_active_anon 0
        nr_zone_inactive_file 123
        nr_zone_active_file 456
Node 0, zone    Normal
  pages free     123456
        min      1234
        low      1543
        high     1852
"#;

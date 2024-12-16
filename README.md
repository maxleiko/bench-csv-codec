Writes `N` rows of `ISO8601,VALUE` where value is a `i64` from `[0..10_000[`

For 10,000,000 records on a Intel i7-8750H (12) @ 4.100GHz and a SATA MX500 SSD (theorical W:510MB/s,R:560MB/s)
```
        algo write_speed  write_took  read_speed   read_took    filesize  total_time
         raw  140.61MB/s       2.80s  140.61MB/s       2.49s    281.23MB       5.29s
     gzip(1)   20.34MB/s       3.54s   20.34MB/s       3.06s     61.03MB       6.60s
     gzip(9)    1.39MB/s      35.74s   24.27MB/s       2.93s     48.54MB      38.67s
         lz4   45.70MB/s       2.93s   45.70MB/s       2.68s     91.40MB       5.60s
        snap   43.07MB/s       2.72s   43.07MB/s       2.63s     86.15MB       5.35s
        zstd   16.85MB/s       3.52s   25.27MB/s       2.71s     50.54MB       6.23s
```

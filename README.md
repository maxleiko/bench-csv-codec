For 50,000,000 records on a Intel i7-8750H (12) @ 4.100GHz and an M2 NVMe:
```
algo    w (MB/s)    r (MB/s)   size (MB)        took
 raw      100.44      117.18     1406.15      27.16s
 lz4       24.05       35.15      456.99      33.30s
snap       21.54       21.54      430.72      41.32s
zstd       10.53       18.05      252.68      38.65s
```

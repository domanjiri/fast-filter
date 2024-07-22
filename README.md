# High throughput and lock-free Ad-Engine over gRPC

A proposal for a lock-free and consistent-performance filter, implemented using [Roaring Bitmaps](https://roaringbitmap.org/) and [ArcSwap](https://docs.rs/arc-swap/1.7.1/arc_swap/docs/performance/index.html). Supposed to be one of the most efficient approaches in read-intensive environments.

NOTE: It's made as a POC for the purpose of qualification. Should not be trusted by any means, nor should it be used as it is!

## TODO
- [ ] Unit & Scenario tests
- [ ] Data Schema for Ads
- [ ] Benchmarks that cover a range of scales (spanning ten to millions of ads)
- [ ] More write-friendly strategy in inventory

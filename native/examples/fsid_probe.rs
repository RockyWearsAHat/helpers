//! Probes FSEvents id semantics: does `FSEventsGetCurrentEventId` advance immediately
//! (kernel-assigned) after a write, or lag behind fseventsd ingestion? The whole-project
//! memo's soundness rests on "immediately".

#[link(name = "CoreServices", kind = "framework")]
extern "C" {
    fn FSEventsGetCurrentEventId() -> u64;
}

fn main() {
    let dir = std::env::temp_dir().join(format!("fsid-probe-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    for round in 0..5 {
        let before = unsafe { FSEventsGetCurrentEventId() };
        let t = std::time::Instant::now();
        std::fs::write(dir.join(format!("f{round}")), b"x").unwrap();
        let mut after = unsafe { FSEventsGetCurrentEventId() };
        let immediate = after > before;
        let mut waited_us = t.elapsed().as_micros();
        if !immediate {
            while after <= before && t.elapsed().as_millis() < 3000 {
                std::thread::yield_now();
                after = unsafe { FSEventsGetCurrentEventId() };
            }
            waited_us = t.elapsed().as_micros();
        }
        println!(
            "round {round}: before={before} after={after} advanced_immediately={immediate} observed_after_us={waited_us}"
        );
    }
    let t = std::time::Instant::now();
    let n = 10_000;
    let mut acc = 0u64;
    for _ in 0..n {
        acc ^= unsafe { FSEventsGetCurrentEventId() };
    }
    println!("call cost: {:.2}µs each (acc {acc})", t.elapsed().as_micros() as f64 / n as f64);
    let _ = std::fs::remove_dir_all(&dir);
}

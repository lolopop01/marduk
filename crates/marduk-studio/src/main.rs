use marduk_ui::Application;

fn main() {
    // Startup banner — printed before the window opens.
    println!();
    println!("  ╔════════════════════════════════════════╗");
    println!("  ║       MARDUK MISSION CONTROL v0.1      ║");
    println!("  ║   wgpu renderer  ·  marduk-ui dsl      ║");
    println!("  ╠════════════════════════════════════════╣");
    println!("  ║  All subsystems initialized.           ║");
    println!("  ║  Awaiting commands from operator...    ║");
    println!("  ╚════════════════════════════════════════╝");
    println!();

    Application::new()
        .title("Marduk Mission Control")
        .size(820.0, 560.0)
        .font("body", load_font())
        .component("Header",   include_str!("../ui/header.mkml"))
        .component("Controls", include_str!("../ui/controls.mkml"))
        // ── SYSTEMS ───────────────────────────────────────────────────────
        .on_event("launch_sequence", || {
            println!();
            println!("  ╔═══════════════════════════════════════╗");
            println!("  ║      LAUNCH SEQUENCE INITIATED        ║");
            println!("  ╠═══════════════════════════════════════╣");
            println!("  ║  T-05  Primary systems check ..  GO   ║");
            println!("  ║  T-04  Fuel pressure nominal  ..  GO  ║");
            println!("  ║  T-03  Navigation locked      ..  GO  ║");
            println!("  ║  T-02  Engine ignition        ..  GO  ║");
            println!("  ║  T-01  Launch commit          ..  GO  ║");
            println!("  ╠═══════════════════════════════════════╣");
            println!("  ║         *** LIFTOFF CONFIRMED ***     ║");
            println!("  ╚═══════════════════════════════════════╝");
            println!();
        })
        .on_event("run_diagnostics", || {
            println!();
            println!("  [DIAGNOSTICS] Running full system check...");
            println!();
            println!("  CPU      ████████████████░░░░  78%   OK");
            println!("  MEMORY   ████████░░░░░░░░░░░░  41%   OK");
            println!("  GPU      ███████████████████░  96%   WARM");
            println!("  NETWORK  ████░░░░░░░░░░░░░░░░  19%   OK");
            println!("  STORAGE  ██████████░░░░░░░░░░  51%   OK");
            println!("  BATTERY  ████████████████████  100%  OK");
            println!();
            println!("  1 warning — GPU running hot. Check airflow.");
            println!();
        })
        .on_event("scan_sector", || {
            println!();
            println!("  [SCAN] Sweeping sector Alpha-7...");
            println!();
            println!("    . . . . . . . * . . . . . . . . .");
            println!("    . . . . . . . . . . @ . . . . . .");
            println!("    . . . # . . . . . . . . . . . . .");
            println!("    . . . . . . . . . . . . . . * . .");
            println!("    . . . . . . . . . . . . . . . . .");
            println!();
            println!("  Objects: 3   Threats: NONE   Anomalies: 1");
            println!("  Object @ at [10,1] — uncharted. Flag for analysis.");
            println!();
        })
        // ── DATA OPS ──────────────────────────────────────────────────────
        .on_event("transmit_data", || {
            println!();
            println!("  [TX] Transmitting data packet...");
            println!();
            println!("  Endpoint  >  192.168.0.1:7777");
            println!("  Payload   >  1,337 bytes");
            println!("  Encoding  >  AES-256-GCM");
            println!("  Checksum  >  0xDEADBEEF (verified)");
            println!("  Latency   >  12 ms");
            println!("  Status    >  SENT OK");
            println!();
        })
        .on_event("analyze_anomaly", || {
            println!();
            println!("  [ANOMALY] Non-standard signature detected!");
            println!();
            println!("  Frequency  :  42.7 THz");
            println!("  Origin     :  Grid [10, 1] sector Alpha-7");
            println!("  Pattern    :  periodic / 3.2s interval");
            println!("  Type       :  UNKNOWN");
            println!("  Risk level :  MODERATE");
            println!();
            println!("  Recommendation: deploy probe, do not engage.");
            println!();
        })
        .on_event("calibrate_sensors", || {
            println!();
            println!("  [CALIBRATE] Sensor array calibration...");
            println!();
            println!("  Sensor 1   PASS   delta = 0.001");
            println!("  Sensor 2   PASS   delta = 0.003");
            println!("  Sensor 3   FAIL   delta = 1.847  (!)");
            println!("  Sensor 4   PASS   delta = 0.002");
            println!("  Sensor 5   PASS   delta = 0.004");
            println!();
            println!("  WARNING: Sensor 3 out of tolerance. Schedule maintenance.");
            println!();
        })
        // ── OPERATIONS ────────────────────────────────────────────────────
        .on_event("engage_warp", || {
            println!();
            println!("  [WARP] Initiating jump drive sequence...");
            println!();
            println!("  Destination   :  Proxima Centauri b");
            println!("  Distance      :  4.24 light years");
            println!("  Jump window   :  open (stable)");
            println!("  Core temp     :  nominal");
            println!();
            println!("                * * * WHOOOOSH * * *");
            println!();
            println!("  Jump complete. Welcome to the neighbourhood.");
            println!();
        })
        .on_event("activate_shields", || {
            println!();
            println!("  [SHIELDS] Defensive array status:");
            println!();
            println!("  Forward    [████████████]  100%  FULL");
            println!("  Aft        [█████████░░░]   78%  OK");
            println!("  Port       [████████████]  100%  FULL");
            println!("  Starboard  [███████░░░░░]   62%  LOW");
            println!();
            println!("  Average integrity: 85%  —  shields holding.");
            println!("  Note: starboard emitter needs recharge.");
            println!();
        })
        .on_event("hail_frequency", || {
            println!();
            println!("  [COMM] Opening hailing frequencies...");
            println!("  Broadcasting on 432.1 MHz...");
            println!();
            println!("  .");
            println!("  . .");
            println!("  . . .");
            println!();
            println!("  Response received:");
            println!("  \"This is Proxima Station — please identify yourself.\"");
            println!();
            println!("  MARDUK: \"We come in peace. Also, we need coffee.\"");
            println!();
        })
        // ── OVERRIDE ──────────────────────────────────────────────────────
        .on_event("emergency_stop", || {
            println!();
            println!("  !!! EMERGENCY STOP ACTIVATED !!!");
            println!();
            println!("  Non-essential systems offline.");
            println!("  Propulsion    >  HALTED");
            println!("  Navigation    >  STANDBY");
            println!("  Weapons       >  SAFED");
            println!("  Life support  >  ACTIVE (protected)");
            println!();
            println!("  Awaiting operator clearance to resume.");
            println!();
        })
        .on_event("reset_systems", || {
            println!();
            println!("  [RESET] Full system reset in progress...");
            println!();
            println!("  Flushing memory banks         DONE");
            println!("  Reinitializing subsystems     DONE");
            println!("  Loading default config        DONE");
            println!("  Verifying sensor array        DONE");
            println!("  Handshaking with core         DONE");
            println!();
            println!("  All systems nominal. Ready for operation.");
            println!();
        })
        .on_event("deploy_payload", || {
            println!();
            println!("  [DEPLOY] Payload deployment pipeline:");
            println!();
            println!("  Build       cargo build --release       OK");
            println!("  Test        cargo test (42 passed)      OK");
            println!("  Package     marduk-v0.1.0-x86_64        OK");
            println!("  Upload      cluster-prod-7  (1.2 MB/s)  OK");
            println!("  Health      GET /healthz -> 200         OK");
            println!("  Smoke       3 / 3 checks passing        OK");
            println!();
            println!("  Deployment successful. Enjoy the new version!");
            println!();
        })
        .on_event("window_close", || std::process::exit(0))
        .run(include_str!("../ui/main.mkml"))
}

fn load_font() -> Vec<u8> {
    [
        "/usr/share/fonts/TTF/DejaVuSans.ttf",
        "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
        "/usr/share/fonts/dejavu/DejaVuSans.ttf",
        "/usr/share/fonts/noto/NotoSans-Regular.ttf",
        "/usr/share/fonts/truetype/noto/NotoSans-Regular.ttf",
    ]
    .iter()
    .find_map(|p| std::fs::read(p).ok())
    .unwrap_or_default()
}

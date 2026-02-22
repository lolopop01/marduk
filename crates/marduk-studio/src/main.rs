use anyhow::Result;

fn main() -> Result<()> {
    let cfg = marduk_engine::window::RuntimeConfig::default();
    let gpu_init = marduk_engine::device::GpuInit::default();

    marduk_engine::window::Runtime::run(cfg, gpu_init, |_ctx, _id, _window, _gpu, _dt| {
        marduk_engine::window::RunControl::Continue
    })
}
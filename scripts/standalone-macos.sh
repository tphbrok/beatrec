# Bundle for release (without xtask, because it's running main.rs now)
cargo build --release && \

# Run the executable with a sample rate of 44.1 kHz
./target/release/beatrec --sample-rate 44100
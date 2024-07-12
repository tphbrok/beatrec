# Beatrec

Beatrec is an open source minimalistic rolling sampler, recording either a single beat or whatever range is set to loop in the DAW (assuming the DAW correctly passes this information on to the plug-in).

![Screenshot](screenshot.png)

Find more information at [tphbrok.github.io/projects/beatrec](tphbrok.github.io/projects/beatrec)

## Issues

Please create an issue in this repository if you encounter a bug or shortcoming, or if you have a wish or suggestion for the plug-in.

## Development

If you clone the repository to a macOS system (like mine), you can use the scripts in the `scripts` folder to build the plug-in from source or to run it in standalone mode.

For other systems or for more instructions and details, see the following sections.

### Build the plug-in from source

To build the plug-in from source, run

```sh
cargo xtask bundle beatrec --release
```

### Run the plug-in in standalone mode from source

To run the plug-in in standalone mode from source, run

```sh
# Bundle for release (without xtask, because it's running main.rs now)
cargo build --release && \

# Run the executable with a sample rate of 44.1 kHz
./target/release/beatrec --sample-rate 44100
```

Note that the standalone instructions do not mention input audio or devices. I mainly use this mode for UI development, and export it to my CLAP plug-ins folder and test it with Bitwig Studio.

# Bundle for release
cargo xtask bundle beatrec --release && \

# Rename CLAP bundle and copy it to the default CLAP plug-ins folder
mv target/bundled/beatrec.clap target/bundled/Beatrec.clap && \
sudo rm -rf /Library/Audio/Plug-ins/CLAP/Beatrec.clap && \
sudo cp -r target/bundled/Beatrec.clap /Library/Audio/Plug-ins/CLAP && \

# Rename VST3 bundle and copy it to the default VST3 plug-ins folder
mv target/bundled/beatrec.vst3 target/bundled/Beatrec.vst3 && \
sudo rm -rf /Library/Audio/Plug-ins/VST3/Beatrec.vst3 && \
sudo cp -r target/bundled/Beatrec.vst3 /Library/Audio/Plug-ins/VST3
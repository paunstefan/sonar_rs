# sonar_rs

A small remote controlled sonar written in Rust.

Currently incomplete until I have a working distance sensor.

## How to use

To build you need to install cargo-make (stock cargo can't currently build different workspace members for different targets) and run `cargo make build`.

The `sonar_rpi` binary has to be run on a Raspberry Pi which is connected to a SG90 servo motor on PWM channel 1 (this can be changed in the code).

`sonar_client` runs on another machine and is the GUI used to control the servo and sensor. Connect to the RPi's address on port 1111 and you can control it.

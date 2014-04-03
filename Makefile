CRATE = libmqtt-2d3cf69a-0.0.dylib

mqtt: mqtt.rs
	rustc --lib $<

example: example.rs
	rustc $<

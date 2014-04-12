CRATE = libmqtt-2d3cf69a-0.0.dylib
OPTS = --out-dir bin/

mqtt: mqtt.rs
	rustc $(OPTS) $<

example: example.rs 
	rustc $(OPTS) -L bin/ $<

test: mqtt.rs
	rustc --test $(OPTS) $< && bin/mqtt

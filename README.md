Collected demos of rust code running on the raspberry pi pico.

Demos are less interesting when they have nothing but an LED to blink. Hence
most of these use an inexpensive [LCD display][lcd], with a
[6 six wire connection][wiring] to the pico.

- [`demo1`](./demo1) - A simple demo with display rendering using the
  [embeddded graphics library][embedg].
- [`blinky-embassy`](./blinky-embassy) - The blinky demo for the embassy async
  framework. Lifted from [here][embassyblink], but as a standalone project.
- [`demo1-embassy`](./demo1-embassy) - same as demo1, but ported to use the
  embassy async framework.
- [`wifi-example`](./wifi-example) - This is the wifi echo server demo lifted
  from [here][embassyblink], but with status shown on the LCD display. Needs a
  pico w.
- [`spi-display-embassy`](./spi-display-embassy) - An LCD display and a touch
  screen, using [this hardware][wiring]. Based upon
  [this example][embassyspidisplay].

# Dev setup

The dev probe is a raspberry pi pico running cmsis-dap firmware. Follow
the setup instructions here:

https://github.com/rp-rs/rp2040-project-template/blob/main/debug_probes.md

Install probe-rs. See the docs for methods, eg:

```
cargo binstall probe-rs
```

On linux, don't forget to setup the [udev rules][udev] to allow access as described
here:

Once all this is sorted, and a target rpi (non wifi version) is wired up
this should work:

```
cd blinky-embassy
cargo run --release
```

[udev]:https://probe.rs/docs/getting-started/probe-setup/#linux%3A-udev-rules
[lcd]: http://www.lcdwiki.com/2.8inch_SPI_Module_ILI9341_SKU:MSP2807
[wiring]: schematics/demo1.pdf
[embedg]: https://docs.rs/embedded-graphics/latest/embedded_graphics
[embassyblink]: https://github.com/embassy-rs/embassy/blob/master/examples/rp/src/bin/blinky.rs
[embassyspidisplay]: https://github.com/embassy-rs/embassy/blob/master/examples/rp/src/bin/spi_display.rs
[cyw43demo]: https://github.com/embassy-rs/cyw43/tree/master/examples/rpi-pico-w

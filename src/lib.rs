use serial::SerialPort;
use protocols::Serial;

pub mod instruments;
pub mod protocols;

fn config_serial<T: SerialPort>(port: &mut T, config: Serial) -> serial::Result<()> {
    port.reconfigure(&|settings| {
        settings.set_baud_rate(config.baud_rate)?;
        settings.set_char_size(config.data_bits);
        settings.set_parity(config.parity);
        settings.set_stop_bits(config.stop_bits);
        settings.set_flow_control(config.flow_control);
        Ok(())
    })
}

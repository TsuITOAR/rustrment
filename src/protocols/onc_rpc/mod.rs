mod xdr;
mod port_mapper;

pub trait OncRpc{
    const PROGRAM:u32;
    const VERSION:u32;
    type Procedure:Into<u32>;
    
}
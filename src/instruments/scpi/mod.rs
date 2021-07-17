pub enum Query {
    ///Identification query
    IDN,
    ///Self-test query
    TST,
    ///Operation complete query
    OPC,
    ///Event status enable query
    ESE,
    ///Event status register query
    ESR,
    ///Service request enable query
    SRE,
    ///Read status byte wuery
    STB,
}

impl super::Query for Query {
    type R = &'static str;
    fn to_bytes(self) -> Self::R {
        match self {
            Query::IDN => "*IDN?",
            Query::TST => "*TST?",
            Query::OPC => "*OPC?",
            Query::ESE => "*ESE?",
            Query::ESR => "*ESR?",
            Query::SRE => "*SRE?",
            Query::STB => "*STB?",
        }
    }
}

pub enum Command {
    ///Reset
    RST,
    ///Operation complete
    OPC,
    ///Wait to complete
    WAI,
    ///Clear status
    CLS,
    ///Event status enable
    ESE,
    ///Service request enable
    SRE,
}

impl super::Command for Command {
    type R = &'static str;
    fn to_bytes(self) -> Self::R {
        match self {
            Command::RST => "*RST",
            Command::OPC => "*OPC",
            Command::WAI => "*WAI",
            Command::ESE => "*ESE",
            Command::CLS => "*CLS",
            Command::SRE => "*SRE",
        }
    }
}

pub trait SCPI {}

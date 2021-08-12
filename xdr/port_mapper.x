struct mapping {
    unsigned int prog;
    unsigned int vers;
    unsigned int prot;
    unsigned int port;
};
const IPPROTO_TCP = 6;      /* protocol number for TCP/IP */
const IPPROTO_UDP = 17;     /* protocol number for UDP/IP */

//struct *pmaplist {
//    mapping map;
//    pmaplist next;
//};

struct call_args {
    unsigned int prog;
    unsigned int vers;
    unsigned int proc;
    opaque args<>;
};

struct call_result {
    unsigned int port;
    opaque res<>;
};

/*

program DEVICE_CORE {
    version DEVICE_CORE_VERSION {
        Create_LinkResp create_link (Create_LinkParms) = 10;
        Device_WriteResp device_write (Device_WriteParms) = 11;
        Device_ReadResp device_read (Device_ReadParms) = 12;
        Device_ReadStbResp device_readstb (Device_GenericParms) = 13;
        Device_Error device_trigger (Device_GenericParms) = 14;
        Device_Error device_clear (Device_GenericParms) = 15;
        Device_Error device_remote (Device_GenericParms) = 16;
        Device_Error device_local (Device_GenericParms) = 17;
        Device_Error device_lock (Device_LockParms) = 18;
        Device_Error device_unlock (Device_Link) = 19;
        Device_Error device_enable_srq (Device_EnableSrqParms) = 20;
        Device_DocmdResp device_docmd (Device_DocmdParms) = 22;
        Device_Error destroy_link (Device_Link) = 23;
        Device_Error create_intr_chan (Device_RemoteFunc) = 25;
        Device_Error destroy_intr_chan (void) = 26;
    } = 1;
} = 0x0607AF;

program DEVICE_INTR {
    version DEVICE_INTR_VERSION {
        void device_intr_srq (Device_SrqParms) = 30;
    }=1;
}= 0x0607B1;


*/
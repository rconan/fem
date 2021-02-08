use super::IO;
use serde;
use serde::Deserialize;
use std::fmt;
use std::ops::{Deref, DerefMut};

macro_rules! fem_io {
    ($io:ident: $($name:expr, $variant:ident),+) => {
        #[derive(Deserialize, Debug)]
        pub enum $io {
            $(#[serde(rename = $name)]
            $variant(Vec<IO>)),+
        }
        impl $io {
            pub fn len(&self) -> usize {
                match self {
                    $($io::$variant(io) => {
                        io.iter().fold(0,|a,x| a + x.is_on() as usize)
                    }),+
                }
            }
        }
        impl Deref for $io {
            type Target = [IO];
            fn deref(&self) -> &Self::Target {
                match self {
                    $($io::$variant(io) => io),+
                }
            }
        }
        impl DerefMut for $io {
            fn deref_mut(&mut self) -> &mut Self::Target {
                match self {
                    $($io::$variant(io) => io),+
                }
            }
        }
        impl fmt::Display for $io {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                match self {
                    $($io::$variant(_) => write!(f,"{:>24}: [{:5}]",$name,self.len())),+
                }
            }
        }
    };
}
fem_io!(Inputs:
        "M1_actuators_segment_1",
        M1ActuatorsSegment1,
        "M1_actuators_segment_2",
        M1ActuatorsSegment2,
        "M1_actuators_segment_3",
        M1ActuatorsSegment3,
        "M1_actuators_segment_4",
        M1ActuatorsSegment4,
        "M1_actuators_segment_5",
        M1actuatorsSegment5,
        "M1_actuators_segment_6",
        M1actuatorsSegment6,
        "M1_actuators_segment_7",
        M1ActuatorsSegment7,
        "M1_distributed_windF",
        M1DistributedWindf,
        "MC_M2_Grav_CS0",
        MCM2GravCS0,
        "MC_M2_PZT_S1_F",
        MCM2PZTS1F,
        "MC_M2_PZT_S2_F",
        MCM2PZTS2F,
        "MC_M2_PZT_S3_F",
        MCM2PZTS3F,
        "MC_M2_PZT_S4_F",
        MCM2PZTS4F,
        "MC_M2_PZT_S5_F",
        MCM2PZTS5F,
        "MC_M2_PZT_S6_F",
        MCM2PZTS6F,
        "MC_M2_PZT_S7_F",
        MCM2PZTS7F,
        "MC_M2_lcl_force_6F",
        MCM2Lcl6F,
        "MC_M2_small_S1_6F",
        MCM2SmallS16F,
        "MC_M2_small_S2_6F",
        MCM2SmallS26F,
        "MC_M2_small_S3_6F",
        MCM2SmallS36F,
        "MC_M2_small_S4_6F",
        MCM2SmallS46F,
        "MC_M2_small_S5_6F",
        MCM2SmallS56F,
        "MC_M2_small_S6_6F",
        MCM2SmallS66F,
        "MC_M2_small_S7_6F",
        MCM2SmallS76F,
        "OSS_AzDrive_F",
        OSSAzDriveF,
        "OSS_BASE_6F",
        OSSBASE6F,
        "OSS_CRING_6F",
        OSSCRING6F,
        "OSS_Cell_lcl_6F",
        OSSCellLcl6F,
        "OSS_ElDrive_F",
        OSSElDriveF,
        "OSS_GIRDrive_F",
        OSSGIRDriveF,
        "OSS_GIR_6F",
        OSSGIR6F,
        "OSS_Grav_CS0",
        OSSGravCS0,
        "OSS_Harpoint_delta_F",
        OSSHarpointDeltaF,
        "OSS_M1_lcl_6F",
        OSSM1Lcl6F,
        "OSS_TopEnd_6F",
        OSSTopEnd6F,
        "OSS_Truss_6F",
        OSSTruss6F
);
fem_io!(Outputs:
        "OSS_AzDrive_D",
        OSSAzDriveD,
        "OSS_ElDrive_D",
        OSSElDriveD,
        "OSS_GIRDrive_D",
        OSSGIRDriveD,
        "OSS_BASE_6D",
        OSSBASE6D,
        "OSS_Hardpoint_D",
        OSSHardpointD,
        "OSS_M1_lcl",
        OSSM1Lcl,
        "OSS_M1_LOS",
        OSSM1LOS,
        "OSS_IMUs_6d",
        OSSIMUs6d,
        "OSS_Truss_6d",
        OSSTruss6d,
        "OSS_Cell_lcl",
        OSSCellLcl,
        "MC_M2_small_S1_6D",
        MCM2SmallS16D,
        "MC_M2_PZT_S1_D",
        MCM2PZTS1D,
        "MC_M2_small_S2_6D",
        MCM2SmallS26D,
        "MC_M2_PZT_S2_D",
        MCM2PZTS2D,
        "MC_M2_small_S3_6D",
        MCM2SmallS36D,
        "MC_M2_PZT_S3_D",
        MCM2PZTS3D,
        "MC_M2_small_S4_6D",
        MCM2SmallS46D,
        "MC_M2_PZT_S4_D",
        MCM2PZTS4D,
        "MC_M2_small_S5_6D",
        MCM2SmallS56D,
        "MC_M2_PZT_S5_D",
        MCM2PZTS5D,
        "MC_M2_small_S6_6D",
        MCM2SmallS66D,
        "MC_M2_PZT_S6_D",
        MCM2PZTS6D,
        "MC_M2_small_S7_6D",
        MCM2SmallS76D,
        "MC_M2_PZT_S7_D",
        MCM2PZTS7D,
        "MC_M2_lcl_6D",
        MCM2Lcl6D,
        "MC_M2_LOS_6D",
        MCM2LOS6D,
        "M1_surfaces_d",
        M1SurfacesD,
        "M1_edge_sensors",
        M1EdgeSensors,
        "M1_segment_1_axial_d",
        M1Segment1AxialD,
        "M1_segment_2_axial_d",
        M1Segment2AxialD,
        "M1_segment_3_axial_d",
        M1Segment3AxialD,
        "M1_segment_4_axial_d",
        M1Segment4AxialD,
        "M1_segment_5_axial_d",
        M1Segment5AxialD,
        "M1_segment_6_axial_d",
        M1Segment6AxialD,
        "M1_segment_7_axial_d",
        M1Segment7AxialD
);

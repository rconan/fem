use super::{IOData, IO};
use serde;
use serde::Deserialize;
use std::fmt;
use std::ops::{Deref, DerefMut};

macro_rules! fem_io {
    ($io:ident: $($name:expr, $variant:ident),+) => {
        #[derive(Deserialize, Debug, Clone)]
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
            pub fn get_by<F,T>(&self, pred: F) -> Vec<T>
                where
                F: Fn(&IOData) -> Option<T> + Copy,
            {
                match self {
                    $($io::$variant(io) => {
                        io.iter().filter_map(|x| x.get_by(pred)).collect()
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
                    $($io::$variant(io) => {
                        let mut cs: Vec<_> = io.iter().filter_map(|x| match x {
                            IO::On(data) => data.properties.cs_label.as_ref(),
                            IO::Off(_) => None
                        }).collect();
                        cs.sort();
                        cs.dedup();
                        if cs.len()>1 {
                            write!(f,"{:>24}: [{:5}]",$name,self.len())
                        } else {
                            write!(f,"{:>24}: [{:5}] {:?}",$name,self.len(),cs)
                        }}),+
                }
            }
        }
    };
}
fem_io!(Inputs:
        "slew_torques",
        SlewTorques,
        "MC_M2_TE_6F",
        MCM2TE6F,
        "MC_M2_TEIF_6F",
        MCM2TEIF6F,
        "MC_M2_SmHex_F",
        MCM2SmHexF,
        "MC_M2_PMA_1F",
        MCM2PMA1F,
        "MC_M2_CP_6F",
        MCM2CP6F,
        "MC_M2_RB_6F",
        MCM2RB6F,
        "MC_ASM_COG_6F",
        MCASMCOG6F,
        "MC_ASM_COG_6D",
        MCASMCOG6D,
        "OSS_M1_fans_lcl_6F",
        OSSM1FansLcl6F,
        "OSS_payloads_6F",
        OSSPayloads6F,
        "OSS_TrussTEIF_6F",
        OSSTrussTEIF6f,
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
        "MC_M2_lcl_6F",
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
        OSSTruss6F,
        "OSS_AzDrive_Torque",
        OSSAzDriveTorque,
        "OSS_ElDrive_Torque",
        OSSElDriveTorque,
        "OSS_RotDrive_Torque",
        OSSRotDriveTorque,
         "MC_M2_S1_VC_delta_F",
         MCM2S1VCDeltaF,
    "MC_M2_S1_fluid_damping_F",
    MCM2S1FluidDampingF,
         "MC_M2_S2_VC_delta_F",
         MCM2S2VCDeltaF,
    "MC_M2_S2_fluid_damping_F",
    MCM2S2FluidDampingF,
         "MC_M2_S3_VC_delta_F",
         MCM2S3VCDeltaF,
    "MC_M2_S3_fluid_damping_F",
    MCM2S3FluidDampingF,
         "MC_M2_S4_VC_delta_F",
         MCM2S4VCDeltaF,
    "MC_M2_S4_fluid_damping_F",
    MCM2S4FluidDampingF,
         "MC_M2_S5_VC_delta_F",
         MCM2S5VCDeltaF,
    "MC_M2_S5_fluid_damping_F",
    MCM2S5FluidDampingF,
         "MC_M2_S6_VC_delta_F",
         MCM2S6VCDeltaF,
    "MC_M2_S6_fluid_damping_F",
    MCM2S6FluidDampingF,
         "MC_M2_S7_VC_delta_F",
         MCM2S7VCDeltaF,
    "MC_M2_S7_fluid_damping_F",
    MCM2S7FluidDampingF
);
fem_io!(Outputs:
        "OSS_TrussIF_6D",
        OSSTrussIF6D,
        "OSS_GIR_6d",
        OSSGIR6D,
        "OSS_CRING_6d",
        OSSCRING6D,
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
        M1Segment7AxialD,
        "M2_segment_1_axial_d",
        M2segment1axiald,
        "M2_segment_2_axial_d",
        M2segment2axiald,
        "M2_segment_3_axial_d",
        M2segment3axiald,
        "M2_segment_4_axial_d",
        M2segment4axiald,
        "M2_segment_5_axial_d",
        M2segment5axiald,
        "M2_segment_6_axial_d",
        M2segment6axiald,
        "M2_segment_7_axial_d",
        M2segment7axiald,
    "MC_M2_PMA_1D",
        MCM2PMA1D,
    "MC_M2_RB_6D",
        MCM2RB6D,
        "MC_M2_CP_6D",
        MCM2CP6D,
        "MC_M2_CP_1D",
        MCM2CP1D,
        "MC_M2_SmHex_D",
        MCM2SmHexD,
        "MC_M2_lcl",
        MCM2lcl,
        "M2_edge_sensors",
        M2edgesensors,
        "MC_M2_TEIF_6D",
        MCM2TEIF6D,
        "MC_M2_TE_6D",
        MCM2TE6D,
        "MC_ASM_COG_6D",
        MCASMCOG6D,
        "OSS_M1_fans_lcl_6D",
        OSSM1FansLcl6D,
        "OSS_payloads_6D",
        OSSPayloads6D,
        "M2_reference_body_1_axial_d",
        M2ReferenceBody1AxialD,
        "M2_reference_body_2_axial_d",
        M2ReferenceBody2AxialD,
        "M2_reference_body_3_axial_d",
        M2ReferenceBody3AxialD,
        "M2_reference_body_4_axial_d",
        M2ReferenceBody4AxialD,
        "M2_reference_body_5_axial_d",
        M2ReferenceBody5AxialD,
        "M2_reference_body_6_axial_d",
        M2ReferenceBody6AxialD,
        "M2_reference_body_7_axial_d",
        M2ReferenceBody7AxialD,
        "OSS_AzEncoder_Angle",
        OSSAzEncoderAngle,
        "OSS_ElEncoder_Angle",
        OSSElEncoderAngle,
        "OSS_RotEncoder_Angle",
        OSSRotEncoderAngle,
    "MC_M2_S1_VC_delta_D",
    MCM2S1VCDeltaD,
    "MC_M2_S2_VC_delta_D",
    MCM2S2VCDeltaD,
    "MC_M2_S3_VC_delta_D",
    MCM2S3VCDeltaD,
    "MC_M2_S4_VC_delta_D",
    MCM2S4VCDeltaD,
    "MC_M2_S5_VC_delta_D",
    MCM2S5VCDeltaD,
    "MC_M2_S6_VC_delta_D",
    MCM2S6VCDeltaD,
    "MC_M2_S7_VC_delta_D",
    MCM2S7VCDeltaD
);

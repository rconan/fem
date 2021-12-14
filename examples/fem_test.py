import sys
import numpy as np
sys.path.append('/home/rconan/Dropbox/AWS/SIMCEO')
from Telescope.FEM import FEM

fem = FEM(second_order_filename="/home/rconan/Dropbox/AWS/SIMCEO/dos/HFWind/MT_FSM_locked.mat")
inputs=['OSS_M1_lcl_6F', 
        'OSS_Cell_lcl_6F', 
        'OSS_CRING_6F', 
        'OSS_TopEnd_6F', 
        'OSS_GIR_6F', 
        'OSS_Truss_6F', 
        'MC_M2_lcl_force_6F']
outputs= ['OSS_M1_lcl','MC_M2_lcl_6D']
#inputs=['OSS_Truss_6F']
#outputs=['MC_M2_lcl_6D']
fem.reduce(inputs=inputs,outputs=outputs)
A,B,C = fem.state_space(dt=1/2000)
data = np.load("RefinedTelescope_80hz_from_start.pkl",allow_pickle=True)
fem.state.update({'u':np.zeros(fem.N_INPUTS), 
                  'y':np.zeros(fem.N_OUTPUTS), 
                  'A':A,'B':B,'C':C,'D':None, 
                  'x':np.zeros(A.shape[1]), 
                  'step':0})

u = {x[0]:np.zeros(x[1]) for x in fem.INPUTS}
#u['OSS_Truss_6F'][0] = 1
y = []
for k in range(len(data['time'])):
    for key in inputs:
        u[key][...] = np.asarray(data[key][k]).ravel()
    fem.Update(**u)
    y += [np.hstack([fem.Outputs(**{'outputs':outputs})[key].copy() for key in outputs])]
    #print("y:",y[-1])

from scipy.io import loadmat
import pickle
import numpy as np

data = loadmat("modal_state_space_model_2ndOrder.rs.mat")

fem_inputs = data["fem_inputs"]
ins_dict = {}
for input in fem_inputs.dtype.names:

    n_in = fem_inputs[input][0,0].size

    in_dict = []
    for k in range(n_in):
        d = {}
        for l in ["types","exciteIDs","descriptions","indices"]:
            d[l] = fem_inputs[input][0,0][l][0,k].flatten().tolist()
            if l in ["types","descriptions"]:
                d[l] = bytes( d[l]).decode()
            if l in ["exciteIDs","indices"]:
                    d[l] = fem_inputs[input][0,0][l][0,k].flatten().astype(np.uint32).tolist()
        fem_properties = fem_inputs[input][0,0]["properties"][0,k]
        d['properties'] = {x:fem_properties[0,0][x].flatten().tolist()
                           for x in fem_properties.dtype.names}
        d['properties']['csLabel'] =  bytes(d['properties']['csLabel']).decode()
        in_dict.append(d)

    ins_dict[input] = in_dict

fem_outputs = data["fem_outputs"]
outs_dict = {}
for input in fem_outputs.dtype.names:

    n_out = fem_outputs[input][0,0].size

    out_dict = []
    for k in range(n_out):
        d = {}
        for l in ["types", "descriptions","indices"]:
            d[l] = fem_outputs[input][0,0][l][0,k].flatten().tolist()
            if l in ["types","descriptions"]:
                d[l] = bytes( d[l]).decode()
            if l in ["indices"]:
                d[l] = fem_outputs[input][0,0][l][0,k].flatten().astype(np.uint32).tolist()
        fem_properties = fem_outputs[input][0,0]["properties"][0,k]
        d['properties'] = {x:fem_properties[0,0][x].flatten().tolist()
                           for x in fem_properties.dtype.names}
        try:
            d['properties']['csLabel'] =  bytes(d['properties']['csLabel']).decode()
        except:
            pass
        out_dict.append(d)

    outs_dict[input] = out_dict

"""
with open("inputs.pkl", "wb") as f:
    pickle.dump(ins_dict, f)
with open("outputs.pkl", "wb") as f:
    pickle.dump(outs_dict, f)
"""

fem = {"modelDescription": bytes(data["modelDescription"].flatten().tolist()).decode(),
       "inputs":ins_dict, "outputs":outs_dict,
       "eigenfrequencies": data["eigenfrequencies"].flatten().tolist(),
       "inputs2ModalF": data["inputs2ModalF"].flatten().tolist(),
       "modalDisp2Outputs": data["modalDisp2Outputs"].flatten().tolist(),
       "proportionalDampingVec": data["proportionalDampingVec"].flatten().tolist()}
with open("modal_state_space_model_2ndOrder.pkl", "wb") as f:
    pickle.dump(fem, f)

import pickle
import numpy as np

import mat73
try:
    data = mat73.loadmat('modal_state_space_model_2ndOrder.rs.mat')
except:
    data = mat73.loadmat('static_reduction_model.rs.mat')

fem_inputs = data["fem_inputs"]
ins_dict = {}
for input in fem_inputs:

    n_in = len(fem_inputs[input])

    in_dict = []
    for k in range(n_in):
        d = {}
        for l in ["types","exciteIDs","descriptions","indices"]:
            d[l] = fem_inputs[input][k][l].flatten().tolist()
            if l in ["types","descriptions"]:
                d[l] = bytes( d[l]).decode()
            if l in ["exciteIDs","indices"]:
                    d[l] = fem_inputs[input][k][l].flatten().astype(np.uint32).tolist()
        fem_properties = fem_inputs[input][k]["properties"]
        d['properties'] = {x:fem_properties[x].flatten().tolist()
                           for x in fem_properties}
        d['properties']['nodeID'] = fem_properties['nodeID'].astype(np.uint32).flatten().tolist()
        try:
            d['properties']['csNumber'] = fem_properties['csNumber'].astype(np.uint32).flatten().tolist()
        except:
            pass
        d['properties']['csLabel'] =  bytes(d['properties']['csLabel']).decode()
        in_dict.append(d)

    ins_dict[input] = in_dict

fem_outputs = data["fem_outputs"]
outs_dict = {}
for input in fem_outputs:

    n_out = len(fem_outputs[input])

    out_dict = []
    for k in range(n_out):
        d = {}
        for l in ["types", "descriptions","indices"]:
            d[l] = fem_outputs[input][k][l].flatten().tolist()
            if l in ["types","descriptions"]:
                d[l] = bytes( d[l]).decode()
            if l in ["indices"]:
                d[l] = fem_outputs[input][k][l].flatten().astype(np.uint32).tolist()
        fem_properties = fem_outputs[input][k]["properties"]
        d['properties'] = {x:fem_properties[x].flatten().tolist()
                           for x in fem_properties if fem_properties[x] is not None}
        d['properties']['nodeID'] = fem_properties['nodeID'].astype(np.uint32).flatten().tolist()
        try:
            d['properties']['csNumber'] = fem_properties['csNumber'].astype(np.uint32).flatten().tolist()
        except:
            pass
        try:
            d['properties']['component'] = fem_properties['component'].astype(np.int32).flatten().tolist()
        except:
            pass
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
inlist = [{k:v} for (k,v) in ins_dict.items()] 
outlist = [{k:v} for (k,v) in outs_dict.items()]
if  "eigenfrequencies" in data and "inputs2ModalF" in data and "modalDisp2Outputs" in data and "proportionalDampingVec" in data:
    fem = {"modelDescription": bytes(data["modelDescription"].flatten().tolist()).decode("utf-8","ignore"),
           "inputs":inlist, "outputs":outlist,
           "eigenfrequencies": data["eigenfrequencies"].flatten().tolist(),
           "inputs2ModalF": data["inputs2ModalF"].flatten().tolist(),
           "modalDisp2Outputs": data["modalDisp2Outputs"].flatten().tolist(),
           "proportionalDampingVec": data["proportionalDampingVec"].flatten().tolist()}
    with open("modal_state_space_model_2ndOrder.73.pkl", "wb") as f:
        pickle.dump(fem, f)
elif "gainMatrix" in data:
    fem = {"modelDescription": bytes(data["modelDescription"].flatten().tolist()).decode("utf-8","ignore"),
           "inputs":inlist, "outputs":outlist,
           "eigenfrequencies": [],
           "inputs2ModalF": [],
           "modalDisp2Outputs": [],
           "proportionalDampingVec": [],
           "gainMatrix": data["gainMatrix"].flatten().tolist()}
    with open("static_reduction_model.73.pkl", "wb") as f:
        pickle.dump(fem, f)
else:
    print("not a suitable model")

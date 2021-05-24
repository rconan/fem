function table2structure(datapath)
%%
%clear
load(fullfile(datapath,"modal_state_space_model_2ndOrder"))
%%
s  = table2struct(inputTable);
ws = struct();
for j=1:length(s)
    data = s(j);
    wdata = struct();
    for i=1:data.size
        wdata(i).types = uint32(data.types{i});
        wdata(i).exciteIDs = data.exciteIDs(i);
        wdata(i).descriptions = uint32(data.descriptions{i});
        wdata(i).indices = data.indices(i);
        properties = data.properties{i};
        properties.csLabel = uint32(properties.csLabel{1});
        wdata(i).properties = properties;
    end
    ws.(inputTable.Row{j}) = wdata;
end
fem_inputs = ws;
%%
s  = table2struct(outputTable);
ws = struct();
for j=1:length(s)
    data = s(j);
    wdata = struct();
    for i=1:data.size
        wdata(i).types = uint32(data.types{i});
        wdata(i).descriptions = uint32(data.descriptions{i});
        wdata(i).indices = data.indices(i);
        properties = data.properties{i};
        if isfield(properties,'csLabel')
            properties.csLabel = uint32(properties.csLabel{1});
        end
        wdata(i).properties = properties;
    end
    ws.(outputTable.Row{j}) = wdata;
end
fem_outputs = ws;
%%
if iscell(modelDescription) 
    modelDescription = uint32(modelDescription{1});
else
    if isstring(modelDescription)
        modelDescription = uint32(char(modelDescription));
    else
        modelDescription = uint32(modelDescription);
    end
end
%%
save(fullfile(datapath,'modal_state_space_model_2ndOrder.rs.mat'),...
    'modelDescription',...
    'fem_inputs',...
    'fem_outputs',...
    'eigenfrequencies',...
    'inputs2ModalF',...
    'modalDisp2Outputs',...
    'proportionalDampingVec',...
    '-v7.3')

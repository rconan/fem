function table2structure(varargin)
%%
%clear
datapath = varargin{1};
if nargin>1
    file = fullfile(datapath,varargin{2});
else
    file = fullfile(datapath,"modal_state_space_model_2ndOrder");
end
load(file);

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
    modelDescription = uint16(modelDescription{1});
else
    if isstring(modelDescription)
        modelDescription = uint16(char(modelDescription));
    else
        modelDescription = uint16(modelDescription);
    end
end
modelDescription(modelDescription>255) = 255;
%%
if exist('eigenfrequencies','var') && ...
        exist('inputs2ModalF','var') && ...
        exist('modalDisp2Outputs','var') && ...
        exist('proportionalDampingVec')
    save(fullfile(datapath,'modal_state_space_model_2ndOrder.rs.mat'),...
        'modelDescription',...
        'fem_inputs',...
        'fem_outputs',...
        'eigenfrequencies',...
        'inputs2ModalF',...
        'modalDisp2Outputs',...
        'proportionalDampingVec',...
        '-v7.3')
elseif exist('gainMatrix') 
    try
        save(fullfile(datapath,'static_reduction_model.rs.mat'),...
            'modelDescription',...
            'fem_inputs',...
            'fem_outputs',...
            'gainMatrixMountControlled',...
            '-v7.3')
    catch
        save(fullfile(datapath,'static_reduction_model.rs.mat'),...
            'modelDescription',...
            'fem_inputs',...
            'fem_outputs',...
            'gainMatrix',...
            '-v7.3')
    end
else
    disp("The model is neither a 2nd order state space model nor a static reduction model.")
end

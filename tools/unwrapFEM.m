
function unwrapFEM(path_to_model, varargin)
% unwrapFEM(path_to_model, (optional) destinationFolder)
% if destinationFolder is ommitted new files are saved in the same location

%modelFile = string(modelFile);
%assert(isfile(modelFile))
%[folder, filename, extension] = fileparts(modelFile);
%assert(matches(extension, ".mat"), "modelFile is not .mat file")

if nargin < 2
  destinationFolder = path_to_model;
else
  destinationFolder = varargin{1};
  if ~isfolder(destinationFolder)
    assert(mkdir(destinationFolder))
  end
end
folder = destinationFolder;
destinationFolder = fullfile(destinationFolder,"rust");
mkdir(destinationFolder)

filename = "modal_state_space_model_2ndOrder";
sprintf("loading %s", filename);
contents = load(fullfile(path_to_model,filename+".mat"));
assert(~isempty(contents))
names = fieldnames(contents);
assert(ismember("inputTable", names), "inputTable is missing from modelFile")
assert(ismember("outputTable", names), "outputTable is missing from modelFile")
assert(numel(names) > 2, "modelFile does not seem to contain matrices")

in_file = fullfile(destinationFolder, filename + "_in.parquet");
writeTable(contents.inputTable, in_file)
out_file = fullfile(destinationFolder, filename + "_out.parquet");
contents.outputTable.Properties.RowNames(cellfun(@(x)  matches(x,'MC_M2_lcl'),...
    contents.outputTable.Properties.RowNames)) = {'MC_M2_lcl_6D'};
writeTable(contents.outputTable, out_file)
contents = rmfield(contents, ["inputTable", "outputTable"]);

mat_file = fullfile(destinationFolder, filename + "_mat.mat");
inputs2ModalF_file = fullfile(destinationFolder,"inputs2ModalF");
modalDisp2Outputs_file = fullfile(destinationFolder,"modalDisp2Outputs");
static_gain_mat_file = fullfile(destinationFolder,"static_gain");

inputs2ModalF = contents.inputs2ModalF';
contents = rmfield(contents,'inputs2ModalF');
modalDisp2Outputs = contents.modalDisp2Outputs';
contents = rmfield(contents,'modalDisp2Outputs');

static_model_path = fullfile(path_to_model,'static_reduction_model.mat');
if exist(static_model_path,'file')
    sprintf("loading %s", static_model_path);
    static_model = load(static_model_path);
    if isfield(static_model,"gainMatrixMountControlled")
        static_gain = static_model.gainMatrixMountControlled';
        clearvars('static_model')
        check_size_save(static_gain_mat_file,static_gain,"static_gain");
    else
        static_gain = static_model.gainMatrix';
        clearvars('static_model')   
        check_size_save(static_gain_mat_file,static_gain,"static_gain");
    end
    clearvars('static_gain')
    sprintf("writing %s", mat_file);
    save(mat_file, '-struct',...
        'contents','eigenfrequencies','proportionalDampingVec',...
        'modelDescription')
    clearvars('contents')

    check_size_save(inputs2ModalF_file,...
        inputs2ModalF,"inputs2ModalF");
    clearvars('inputs2ModalF')

    check_size_save(modalDisp2Outputs_file,...
        modalDisp2Outputs,"modalDisp2Outputs");
    clearvars('modalDisp2Outputs')

    sprintf("zipping %s", destinationFolder);
    zip(fullfile(folder, filename + ".zip"),destinationFolder)
else
    save(mat_file, '-struct','contents', ...
        'eigenfrequencies','proportionalDampingVec','modelDescription')
    clearvars('contents')

    check_size_save(inputs2ModalF_file,...
        inputs2ModalF,"inputs2ModalF");
    clearvars('inputs2ModalF')

    check_size_save(modalDisp2Outputs_file,...
        modalDisp2Outputs,"modalDisp2Outputs");
    clearvars('modalDisp2Outputs')

    sprintf("zipping %s", destinationFolder);
    zip(fullfile(folder, filename + ".zip"),destinationFolder)
end

rmdir(destinationFolder,'s');

  function writeTable(t, fn)
    % save table in parquet format
    n = sum(t.size);
    group = strings(n,1);
    description = strings(n,1);
    index = zeros(n,1);
    X = zeros(n,1);
    Y = zeros(n,1);
    Z = zeros(n,1);
    csLabel = strings(n,1);
    rowNames = t.Properties.RowNames;
    rr = 1;
    for ii = 1:height(t)
      for jj = 1:t.size(ii)
        group(rr) = rowNames(ii);
        index(rr) = t.indices{ii}(jj);
        description(rr) = t.descriptions{ii}(jj);        
        csLabel(rr) = t.properties{ii}{jj}.csLabel(1);
        try
            X(rr) = t.properties{ii}{jj}.location(1,1);
            Y(rr) = t.properties{ii}{jj}.location(1,2);
            Z(rr) = t.properties{ii}{jj}.location(1,3);
        catch 
            warning("missing location property for %s",csLabel(rr));
            X(rr) = 0;
            Y(rr) = 0;
            Z(rr) = 0;
        end
        rr = rr + 1;
      end
    end
    tnew = table(group, index, description, X, Y, Z, csLabel, 'VariableNames', ...
      ["group", "index", "description", "X", "Y", "Z", "csLabel"]);
    parquetwrite(fn, tnew)
  end

    function mat_file = check_size_save(mat_file,v,name)
        s = whos(name);
        limit = 2^30*2;
        if s.bytes>limit 
            [n,m] = size(v);
            b = s.bytes;
            mm = m;
            count = 1;
            while b>limit
                mm = ceil(mm/2);
                b = mm*n*8;
                count = count * 2;
            end
            i0 = 1;
            mkdir(fullfile(mat_file+'.mat'))
            for i = 1:count
                i1 = i0 + mm - 1;
                if i1>m
                    i1 = m;
                end
                slice = v(:,i0:i1);
                file_name = fullfile(mat_file+'.mat',sprintf('slice_%d',i));
                sprintf("writing %s", file_name);
                save(file_name,"slice");
                i0 = i1 + 1;
            end
        else
            sprintf("writing %s", mat_file);
            save(mat_file,name);
        end
    end

end


function unwrapFEM(modelFile, varargin)
% unwrapFEM(modelFilename, (optional) destinationFolder)
% if destinationFolder is ommitted new files are saved in the same location
modelFile = string(modelFile);
assert(isfile(modelFile))
[folder, filename, extension] = fileparts(modelFile);
assert(matches(extension, ".mat"), "modelFile is not .mat file")
if nargin < 2
  destinationFolder = folder;
else
  destinationFolder = varargin{1};
  if ~isfolder(destinationFolder)
    assert(mkdir(destinationFolder))
  end
end
contents = load(modelFile);
assert(~isempty(contents))
names = fieldnames(contents);
assert(ismember("inputTable", names), "inputTable is missing from modelFile")
assert(ismember("outputTable", names), "outputTable is missing from modelFile")
assert(numel(names) > 2, "modelFile does not seem to contain matrices")
in_file = fullfile(destinationFolder, filename + "_in.parquet");
writeTable(contents.inputTable, in_file)
out_file = fullfile(destinationFolder, filename + "_out.parquet");
writeTable(contents.outputTable, out_file)
contents = rmfield(contents, ["inputTable", "outputTable"]);
mat_file = fullfile(destinationFolder, filename + "_mat.mat");
contents.inputs2ModalF = contents.inputs2ModalF';
contents.modalDisp2Outputs = contents.modalDisp2Outputs';
save(mat_file, '-struct',...
    'contents','eigenfrequencies','proportionalDampingVec',...
    'inputs2ModalF','modalDisp2Outputs','modelDescription')

zip(fullfile(destinationFolder, filename + ".zip"),...
    [in_file,out_file,mat_file])

delete(in_file)
delete(out_file)
delete(mat_file)

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
        X(rr) = t.properties{ii}{jj}.location(1,1);
        Y(rr) = t.properties{ii}{jj}.location(1,2);
        Z(rr) = t.properties{ii}{jj}.location(1,3);
        csLabel(rr) = t.properties{ii}{jj}.csLabel(1);
        rr = rr + 1;
      end
    end
    tnew = table(group, index, description, X, Y, Z, csLabel, 'VariableNames', ...
      ["group", "index", "description", "X", "Y", "Z", "csLabel"]);
    parquetwrite(fn, tnew)
  end
end
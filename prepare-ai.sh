#!/usr/bin/env bash
set -euo pipefail
provider=cuda
ai_dir="${XDG_DATA_HOME:-$HOME/.local/share}/facecam-hub/ai"
while (($#)); do
    case "$1" in
        --cpu) provider=cpu; shift ;;
        --cuda) provider=cuda; shift ;;
        --directory) ai_dir=${2:?Informe o diretório}; shift 2 ;;
        --help) echo 'Uso: ./prepare-ai.sh [--cpu|--cuda] [--directory DIRETÓRIO]'; exit 0 ;;
        *) echo "Opção inválida: $1" >&2; exit 2 ;;
    esac
done
if [[ $(uname -s) != Linux || $(uname -m) != x86_64 ]]; then
    echo 'Este preparador fornece os runtimes para Linux x86_64.' >&2
    exit 1
fi
for program in curl jq tar unzip sha256sum; do
    command -v "$program" >/dev/null || { echo "Dependência ausente: $program" >&2; exit 1; }
done
mkdir -p -- "$ai_dir"
ai_dir=$(cd -- "$ai_dir" && pwd)
fetch() {
    local url=$1 destination=$2 checksum=$3
    if [[ ! -f "$destination" ]] || ! printf '%s  %s\n' "$checksum" "$destination" | sha256sum -c --status -; then
        curl --fail --location --retry 2 "$url" -o "$destination.part"
        printf '%s  %s\n' "$checksum" "$destination.part" | sha256sum -c -
        mv -- "$destination.part" "$destination"
    fi
}
fetch 'https://huggingface.co/onnx-community/mediapipe_selfie_segmentation/resolve/be49485c8e027524be38591817fc5cd31bd9d00e/onnx/model.onnx' "$ai_dir/selfie.onnx" '3241ac4ad8aa35bdaf33946776db29f7c283a413aa0b0dacb9483594b4531aad'
curl -fsSL --retry 2 'https://www.apache.org/licenses/LICENSE-2.0.txt' -o "$ai_dir/MODEL-LICENSE-APACHE-2.0.txt"
if [[ "$provider" == cuda ]]; then
    runtime=onnxruntime-linux-x64-gpu-1.22.0
    runtime_sha=2a19dbfa403672ec27378c3d40a68f793ac7a6327712cd0e8240a86be2b10c55
else
    runtime=onnxruntime-linux-x64-1.22.0
    runtime_sha=8344d55f93d5bc5021ce342db50f62079daf39aaafb5d311a451846228be49b3
fi
fetch "https://github.com/microsoft/onnxruntime/releases/download/v1.22.0/$runtime.tgz" "$ai_dir/$runtime.tgz" "$runtime_sha"
tar -xzf "$ai_dir/$runtime.tgz" -C "$ai_dir"
if [[ "$provider" == cuda ]]; then
    for spec in nvidia-cudnn-cu12/9.8.0.87 nvidia-cublas-cu12/12.8.4.1 nvidia-cuda-runtime-cu12/12.8.90 nvidia-curand-cu12/10.3.9.90 nvidia-cufft-cu12/11.3.3.83 nvidia-cuda-nvrtc-cu12/12.8.93; do
        package=${spec%/*}
        curl -fsSL --retry 2 "https://pypi.org/pypi/$spec/json" -o "$ai_dir/$package.json"
        selection='.urls[] | select(.filename | contains("manylinux") and contains("x86_64"))'
        url=$(jq -er "$selection | .url" "$ai_dir/$package.json")
        checksum=$(jq -er "$selection | .digests.sha256" "$ai_dir/$package.json")
        marker="$ai_dir/cuda/.$package.sha256"
        if [[ -f "$marker" && $(cat -- "$marker") == "$checksum" ]]; then
            continue
        fi
        fetch "$url" "$ai_dir/$package.whl" "$checksum"
        unzip -oq "$ai_dir/$package.whl" -d "$ai_dir/cuda"
        printf '%s\n' "$checksum" > "$marker"
        rm -- "$ai_dir/$package.whl"
    done
fi
jq -n --arg provider "$provider" --arg model "$ai_dir/selfie.onnx" --arg runtime "$ai_dir/$runtime/lib/libonnxruntime.so" \
    '{background_blur:true,ai_provider:$provider,ai_model:$model,ai_runtime:$runtime}' > "$ai_dir/effects-$provider.json"
registry_dir="${XDG_DATA_HOME:-$HOME/.local/share}/facecam-hub"
mkdir -p -- "$registry_dir"
registry="$registry_dir/ai-installations.json"
registry_tmp=$(mktemp "$registry_dir/.ai-installations-XXXXXX")
trap 'if [[ -f "$registry_tmp" ]]; then rm -- "$registry_tmp"; fi' EXIT
if [[ -f "$registry" ]]; then
    registry_input="$registry"
else
    registry_input="$ai_dir/.empty-registry.json"
    printf '{}\n' > "$registry_input"
fi
jq --arg provider "$provider" --arg model "$ai_dir/selfie.onnx" --arg runtime "$ai_dir/$runtime/lib/libonnxruntime.so" \
    'if type != "object" then error("Registro de IA inválido; arquivo original preservado") else .[$provider] = {ai_model:$model,ai_runtime:$runtime} end' \
    "$registry_input" > "$registry_tmp"
mv -- "$registry_tmp" "$registry"
printf '\nArquivos de IA preparados em %s\n' "$ai_dir"
printf 'Abra a aplicação:\n./target/release/elgato-virtualcam\n'
printf 'Na aba Effects, os arquivos serão detectados. Ative o desfoque e aplique.\n'
printf 'Ou aplique pela CLI:\n./target/release/elgato-virtualcam --set-effects %q\n' "$ai_dir/effects-$provider.json"

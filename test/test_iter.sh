set -ex

# naga or tint
TRANSLATION_IMPL=${TRANSLATION_IMPL:-"naga"}

RAND_DIR=$(mktemp -u)
TRANSLATION_BIN=${TRANSLATION_BIN:-$(which $TRANSLATION_IMPL)}
KEEP_RES=${KEEP_RES:-0}

mkdir $RAND_DIR

function cleanup {
    if [ $KEEP_RES = 0 ]; then
        rm -rf $RAND_DIR
    fi
}

trap cleanup EXIT

function validate {
    $TRANSLATION_BIN $1 > /dev/null
}

function translate {
    if [ $TRANSLATION_IMPL = "naga" ]; then
        $TRANSLATION_BIN $1 $2
    elif [ $TRANSLATION_IMPL = "tint" ]; then
        $TRANSLATION_BIN $1 -o $2
    fi
}

translate $1 $RAND_DIR/00.wgsl
translate $RAND_DIR/00.wgsl $RAND_DIR/01.spv
spirv-val $RAND_DIR/01.spv
translate $RAND_DIR/01.spv $RAND_DIR/01.wgsl
translate $RAND_DIR/01.wgsl $RAND_DIR/02.spv
spirv-val $RAND_DIR/02.spv
validate $RAND_DIR/02.spv


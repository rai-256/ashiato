#!/usr/bin/env bash
# 携帯の設定のファイル（送り先と資格情報）を、Android のバックアップ（full backup・cloud-backup・device-transfer）から
# 外していることを見る。ファイルの名前はアプリが実際に使う `Config.SETUP_PREFS` から作り、XML の注釈は除いてから見る
# （名前の直書きと注釈の中の行で緑にならない。review/code.md R27）。
set -euo pipefail

ROOT="${ANDROID_BACKUP_RULES_ROOT:-$(cd "$(dirname "$0")/.." && pwd)}"
APP_REL="collector-android/app/src/main"
CONFIG_REL="$APP_REL/kotlin/dev/ashiato/collector/Config.kt"

fail() { echo "error: $*" >&2; return 1; }

# 注釈（`<!-- … -->`。行をまたぐものも）を除いた本文
strip_comments() { perl -0pe 's/<!--.*?-->//gs' "$1"; }

check_rules() {
    local root="$1"
    local manifest="$root/$APP_REL/AndroidManifest.xml" xml_dir="$root/$APP_REL/res/xml" config="$root/$CONFIG_REL"
    [ -f "$manifest" ] || { fail "manifest が無い: $manifest"; return 1; }
    [ -d "$xml_dir" ] || { fail "規則の置き場が無い: $xml_dir"; return 1; }
    [ -f "$config" ] || { fail "Config.kt が無い: $config"; return 1; }

    local prefs
    prefs="$(sed -n 's/.*const val SETUP_PREFS = "\([^"]*\)".*/\1/p' "$config")"
    [ -n "$prefs" ] || { fail "Config.kt の SETUP_PREFS が読めない"; return 1; }
    local rule="domain=\"sharedpref\" path=\"$prefs.xml\""

    local full_name extraction_name
    full_name="$(strip_comments "$manifest" | sed -n 's/.*android:fullBackupContent="@xml\/\([^"]*\)".*/\1/p')"
    extraction_name="$(strip_comments "$manifest" | sed -n 's/.*android:dataExtractionRules="@xml\/\([^"]*\)".*/\1/p')"
    [ -n "$full_name" ] || { fail "manifest の android:fullBackupContent が無い"; return 1; }
    [ -n "$extraction_name" ] || { fail "manifest の android:dataExtractionRules が無い"; return 1; }

    local full="$xml_dir/$full_name.xml" extraction="$xml_dir/$extraction_name.xml"
    [ -f "$full" ] || { fail "full backup 規則が無い: $full"; return 1; }
    [ -f "$extraction" ] || { fail "data extraction 規則が無い: $extraction"; return 1; }
    strip_comments "$full" | grep -Fq "<exclude $rule" \
        || { fail "full backup 規則で $prefs.xml を外していない"; return 1; }
    strip_comments "$full" | grep -Fq '<exclude domain="file" path="update_checks.jsonl"' \
        || { fail "full backup 規則で update_checks.jsonl を外していない"; return 1; }
    local section
    for section in cloud-backup device-transfer; do
        strip_comments "$extraction" | awk -v s="$section" '$0 ~ "<" s "[ >]", $0 ~ "</" s ">"' |
            grep -Fq "<exclude $rule" \
            || { fail "$section で $prefs.xml を外していない"; return 1; }
        strip_comments "$extraction" | awk -v s="$section" '$0 ~ "<" s "[ >]", $0 ~ "</" s ">"' |
            grep -Fq '<exclude domain="file" path="update_checks.jsonl"' \
            || { fail "$section で update_checks.jsonl を外していない"; return 1; }
    done
}

if [ "${1:-}" = "--self-test" ]; then
    work="$(mktemp -d)"
    trap 'rm -rf "$work"' EXIT
    src="$(cd "$(dirname "$0")/.." && pwd)"
    reset() {
        rm -rf "$work/collector-android"
        mkdir -p "$work/$APP_REL/res/xml" "$(dirname "$work/$CONFIG_REL")"
        cp "$src/$APP_REL/AndroidManifest.xml" "$work/$APP_REL/AndroidManifest.xml"
        cp "$src/$APP_REL/res/xml/health_full_backup_content.xml" "$src/$APP_REL/res/xml/health_data_extraction_rules.xml" \
            "$work/$APP_REL/res/xml/"
        cp "$src/$CONFIG_REL" "$work/$CONFIG_REL"
    }
    must_fail() {
        if check_rules "$work" 2>/dev/null; then fail "壊した写しを検出できない: $1"; fi
    }
    full="$work/$APP_REL/res/xml/health_full_backup_content.xml"
    extraction="$work/$APP_REL/res/xml/health_data_extraction_rules.xml"

    echo "== Scenario: 両方のバックアップの規則が設定のファイルを外している"
    echo "== Scenario: 携帯の確かめた記録はバックアップから外れる"
    reset
    check_rules "$work"

    echo "== Scenario: 設定のファイルを外す行を消すと、検査が落ちる"
    reset
    awk '!removed && /path="collector_setup.xml"/ { removed=1; next } { print }' "$full" > "$full.tmp" && mv "$full.tmp" "$full"
    must_fail "full backup の行を消した"
    for section in cloud-backup device-transfer; do
        reset
        awk -v section="$section" '
            $0 ~ "<" section "[ >]" { inside=1 }
            inside && !removed && /path="collector_setup.xml"/ { removed=1; next }
            inside && $0 ~ "</" section ">" { inside=0 }
            { print }
        ' "$extraction" > "$extraction.tmp" && mv "$extraction.tmp" "$extraction"
        must_fail "$section の行を消した"
    done

    # 除外の行を注釈にした写し（R27）
    for target in "$full" "$extraction"; do
        reset
        perl -pi -e 's{(<exclude domain="sharedpref" path="collector_setup.xml"\s*/>)}{<!-- $1 -->}g' "$target"
        must_fail "$(basename "$target") の除外を注釈にした"
    done

    # アプリの設定のファイルの名前を変えた写し（R27）
    reset
    sed -i 's/const val SETUP_PREFS = "collector_setup"/const val SETUP_PREFS = "collector_setup_renamed"/' "$work/$CONFIG_REL"
    grep -q 'collector_setup_renamed' "$work/$CONFIG_REL" || fail "self-test: SETUP_PREFS を書き換えられない"
    must_fail "SETUP_PREFS の名前を変えた"
    echo "android backup rules self-test: PASS"
    exit 0
fi

[ "$#" -eq 0 ] || fail "使い方: $0 [--self-test]"
check_rules "$ROOT"
echo "android backup rules: PASS"

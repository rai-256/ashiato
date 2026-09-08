# Android のビルドに要る道具の場所。**sudo 無しで ~/.local/opt に入れてある。**
#   . ./tools/android-env.sh
# 消したいときは rm -rf ~/.local/opt/{jdk21,android-sdk} だけでよい。
export JAVA_HOME="$HOME/.local/opt/jdk21"
export ANDROID_HOME="$HOME/.local/opt/android-sdk"
export ANDROID_SDK_ROOT="$ANDROID_HOME"
export PATH="$JAVA_HOME/bin:$ANDROID_HOME/cmdline-tools/latest/bin:$ANDROID_HOME/platform-tools:$PATH"

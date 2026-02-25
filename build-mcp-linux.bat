@echo off
echo Building glucose-tracker-mcp for Linux (musl static)...

docker build -f Dockerfile.mcp -t glucose-mcp-build . || goto :error

docker create --name glucose-mcp-tmp glucose-mcp-build || goto :error
docker cp glucose-mcp-tmp:/glucose-tracker-mcp glucose-tracker-mcp-linux || goto :error
docker rm glucose-mcp-tmp

echo.
echo Done! Binary: glucose-tracker-mcp-linux
goto :eof

:error
echo Build failed.
docker rm glucose-mcp-tmp 2>nul
exit /b 1

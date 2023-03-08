# Hud Hub [![Test](https://github.com/IohannRabeson/hudhub/actions/workflows/rust.yml/badge.svg)](https://github.com/IohannRabeson/hudhub/actions/workflows/rust.yml)

HUD manager for Team Fortress 2.

### Testing mode
When enabled the application will install HUDs in a temporary directory 
deleted when the application quits. This is particularly useful to run `hudhub` 
in isolation, or to run the application without having Steam and Team Fortress
installed on your machine, but still be able to install HUDs on disk.  

This mode is disabled by default. To enable it, pass the flag `--testing-mode`
when running `hudhub`.

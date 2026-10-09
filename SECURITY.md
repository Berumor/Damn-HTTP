# Security policy

## Supported versions

Damn HTTP is in early development and has no releases yet. Once releases exist, security fixes will be made for the latest stable release and the current beta.

## Reporting a vulnerability

Please do not open a public issue for a security problem.

Report it privately through GitHub: on the repository page, open the **Security** tab and choose **Report a vulnerability**. If that option is not available, contact the maintainer, [@Berumor](https://github.com/Berumor), through the contact details on their GitHub profile, and say only that you have a security report until a private channel is agreed.

Please include what you found, how to reproduce it, the affected version or commit, and your operating system.

You will get an acknowledgement as soon as the maintainer has read the report. This is a volunteer project, so there is no guaranteed response time. Please allow time for a fix before disclosing publicly; you will be credited in the release notes unless you prefer otherwise.

## What matters most here

- A secret value (a variable marked secret, or a local value) ending up in a committed file, a log or an error message.
- The webview reaching the filesystem, the shell or the network outside the app's own commands.
- A workspace file or an imported file causing code execution, or a read or write outside the workspace folder.
- Git commands being run with attacker-controlled arguments.

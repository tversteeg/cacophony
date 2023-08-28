from typing import Optional
from os import chdir, getcwd
from subprocess import check_output, CalledProcessError
from pathlib import Path
from ftplib import FTP, error_perm
import re
from github import Repository, Github
import discord


class DiscordBot(discord.Client):
    """
    Post the changelog and change the server icon.
    """

    async def on_ready(self):
        discord_text = Path("credentials/discord.txt").read_text()
        # Connect to the Discord channel.
        channel = self.get_channel(int(re.search("channel_id=(.*)", discord_text).group(1)))
        # Get the changelog.
        post = re.search("(# " + VERSION.replace(".", r"\.") + r"((.|\n)*?))^# ",
                         Path("changelog.md").read_text(encoding="utf-8"),
                         flags=re.MULTILINE).group(2).strip()
        post = f"**Update {VERSION}**\n{post}"
        # Post.
        await channel.send(post)
        # Quit.
        await self.close()


def upload_directory(ftp: FTP, folder: str = None) -> None:
    if folder is not None:
        chdir(folder)
        ftp.cwd(folder)
    for fi in Path(getcwd()).resolve().iterdir():
        if fi.is_file():
            with fi.open("rb") as bs:
                ftp.storbinary(f'STOR {fi.name}', bs)
            print(f"Uploaded: {fi.name}")

    
def ftp_login() -> FTP:
    ftp = FTP("subalterngames.com")
    ftp_credentials = Path("credentials/ftp.txt").read_text().split("\n")
    ftp.login(user=ftp_credentials[0], passwd=ftp_credentials[1])
    print("Logged into FTP")
    return ftp
    

def ftp_website(ftp: FTP) -> None:
    cwd = getcwd()
    root_remote = "subalterngames.com/cacophony"
    ftp.cwd(root_remote)
    print("Set cwd")
    chdir("../html")
    upload_directory(ftp)
    upload_directory(ftp, folder="images")
    upload_directory(ftp, folder="../fonts/noto")
    print("...Done!")
    ftp.cwd("/subalterngames.com/cacophony")
    chdir(cwd)


def ftp_cwd(ftp: FTP, folder: str) -> None:
    try:
        ftp.cwd(folder)
    except error_perm:
        ftp.mkd(folder)
        ftp.cwd(folder)


def get_repo() -> Repository:
    token: str = Path("credentials/github.txt").resolve().read_text(encoding="utf-8").strip()
    return Github(token).get_repo("subalterngames/cacophony")


def get_version() -> str:
    # Compare versions.
    version = re.search(r'version = "(.*?)"', Path("../Cargo.toml").read_text()).group(1)
    try:
        resp = check_output(["git", "describe", "--tags", "--abbrev=0"])
        latest_version = str(resp).strip()
    except CalledProcessError:
        latest_version = None
    if version == latest_version:
        print("Can't upload. Update the version.")
        exit()
    return version


def tag(repo: Repository, version: str) -> None:
    repo.create_git_tag(tag=version, message=version, type="commit", object=repo.get_commits()[0].sha)
    print("Tagged.")


def create_builds(repo: Repository, version: str) -> None:
    # Build the releases.
    workflow = repo.get_workflow(66524374)
    workflow.create_dispatch(ref="main", inputs={"version": version})


f = ftp_login()
ftp_website(f)
f.close()
r = get_repo()
v = get_version()
tag(r, v)
create_builds(r, v)

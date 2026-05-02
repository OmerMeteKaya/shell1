#include "../include/completions.h"
#include <stdlib.h>
#include <string.h>

static const char *git_cmds[] = {
    "add", "branch", "checkout", "cherry-pick", "clone", "commit",
    "diff", "fetch", "init", "log", "merge", "mv", "pull", "push",
    "rebase", "remote", "reset", "restore", "rm", "show", "stash",
    "status", "switch", "tag", "--version", "--help", NULL
};

static const char *apt_cmds[] = {
    "install", "remove", "purge", "update", "upgrade", "autoremove",
    "search", "show", "list", "depends", "rdepends", "clean",
    "autoclean", "--help", NULL
};

static const char *pacman_cmds[] = {
    "-S", "-Ss", "-Si", "-Sw", "-Su", "-Syu", "-Syyu",
    "-R", "-Rs", "-Rns",
    "-Q", "-Qs", "-Qi", "-Ql", "-Qo", "-Qe", "-Qm",
    "-U", "-D", "--needed", "--noconfirm", "--help", NULL
};

static const char *yay_cmds[] = {
    "-S", "-Ss", "-Si", "-Su", "-Syu", "-Syyu",
    "-R", "-Rs", "-Rns",
    "-Q", "-Qs", "-Qi", "-Ql",
    "--aur", "--repo", "--devel", "--needed",
    "--noconfirm", "--help", NULL
};

static const char *paru_cmds[] = {
    "-S", "-Ss", "-Si", "-Su", "-Syu",
    "-R", "-Rs", "-Rns",
    "-Q", "-Qs", "-Qi", "-Ql",
    "--aur", "--repo", "--devel",
    "--noconfirm", "--help", NULL
};

static const char *systemctl_cmds[] = {
    "start", "stop", "restart", "reload", "enable", "disable",
    "status", "is-active", "is-enabled", "list-units", "list-timers",
    "daemon-reload", "poweroff", "reboot", "--help", NULL
};

static const char *systemd_analyze_cmds[] = {
    "time", "blame", "critical-chain", "plot", "dot",
    "verify", "security", NULL
};

static const char *journalctl_cmds[] = {
    "-f", "-e", "-x", "-u", "-b", "-k", "-n",
    "--since", "--until", "--list-boots",
    "--disk-usage", "--vacuum-size", "--vacuum-time",
    "--no-pager", "--help", NULL
};

static const char *docker_cmds[] = {
    "run", "ps", "build", "exec", "stop", "rm", "rmi", "images",
    "pull", "push", "logs", "inspect", "network", "volume",
    "compose", "--help", NULL
};

static const char *ip_cmds[] = {
    "addr", "link", "route", "neigh", "rule",
    "tunnel", "maddr", "monitor", "--help", NULL
};

static const char *make_cmds[] = {
    "all", "clean", "install", "uninstall", "test", "debug",
    "release", "--help", NULL
};

static const char *cargo_cmds[] = {
    "build", "run", "test", "check", "clean", "doc", "new",
    "init", "add", "remove", "update", "publish", "--help", NULL
};

static const char *npm_cmds[] = {
    "install", "uninstall", "run", "start", "test", "build",
    "init", "publish", "update", "list", "audit", "ci", "--help", NULL
};

static const char *pip_cmds[] = {
    "install", "uninstall", "list", "show", "search", "freeze",
    "download", "wheel", "check", "--help", NULL
};

static const char *grep_cmds[] = {
    "-r", "-i", "-v", "-n", "-l", "-c", "-A", "-B", "-C",
    "-E", "-F", "-P", "--include", "--exclude",
    "--color", "-w", "-x", "--help", NULL
};

static const char *find_cmds[] = {
    "-name", "-type", "-size", "-mtime", "-atime",
    "-exec", "-delete", "-print", "-maxdepth", "-mindepth",
    "-iname", "-user", "-group", "-perm", "--help", NULL
};

static const char *tar_cmds[] = {
    "-czf", "-xzf", "-cjf", "-xjf", "-cxf", "-xxf",
    "-tf", "-cf", "-xf", "-v", "--help", NULL
};

static const char *curl_cmds[] = {
    "-X", "-H", "-d", "-o", "-O", "-L", "-I",
    "-u", "-k", "-s", "-v", "--json",
    "--compressed", "--help", NULL
};

static const char *vim_cmds[] = {
    "-R", "-n", "-b", "-d", "-u", "+", "--help", NULL
};

static const char *gcc_cmds[] = {
    "-o", "-c", "-g", "-O0", "-O1", "-O2", "-O3",
    "-Wall", "-Wextra", "-std=c99", "-std=c11",
    "-I", "-L", "-l", "-shared", "-fPIC",
    "-DDEBUG", "--help", NULL
};

static const char *python_cmds[] = {
    "-c", "-m", "-i", "-u", "-v",
    "-W", "--version", "--help", NULL
};

static const char *python_m_cmds[] = {
    "venv", "pip", "http.server", "json.tool",
    "unittest", "pytest", "pdb", NULL
};

static const char *gdb_cmds[] = {
    "-q", "-batch", "-ex", "-x", "-p",
    "--args", "--help", NULL
};

static const char *valgrind_cmds[] = {
    "--leak-check=full", "--show-leak-kinds=all",
    "--track-origins=yes", "--verbose",
    "--log-file=", "--tool=memcheck",
    "--tool=callgrind", "--help", NULL
};

static const char *ffmpeg_cmds[] = {
    "-i", "-c", "-c:v", "-c:a", "-vf", "-af",
    "-b:v", "-b:a", "-r", "-s", "-t", "-ss",
    "-map", "-y", "--help", NULL
};

static const char *rsync_cmds[] = {
    "-a", "-v", "-z", "-r", "-u", "-n",
    "--delete", "--exclude", "--include",
    "--progress", "--partial", "-e", "--help", NULL
};

static const char *ssh_cmds[] = {
    "-l", "-p", "-i", "-L", "-R", "-D", "-N", "-f", "-v", NULL
};

typedef struct {
    const char *cmd;
    const char **subcmds;
} CmdTable;

static const CmdTable cmd_table[] = {
    { "git",             git_cmds              },
    { "apt",             apt_cmds              },
    { "apt-get",         apt_cmds              },
    { "pacman",          pacman_cmds           },
    { "yay",             yay_cmds              },
    { "paru",            paru_cmds             },
    { "systemctl",       systemctl_cmds        },
    { "systemd-analyze", systemd_analyze_cmds  },
    { "journalctl",      journalctl_cmds       },
    { "docker",          docker_cmds           },
    { "ip",              ip_cmds               },
    { "make",            make_cmds             },
    { "cargo",           cargo_cmds            },
    { "npm",             npm_cmds              },
    { "pip",             pip_cmds              },
    { "pip3",            pip_cmds              },
    { "grep",            grep_cmds             },
    { "find",            find_cmds             },
    { "tar",             tar_cmds              },
    { "curl",            curl_cmds             },
    { "vim",             vim_cmds              },
    { "nvim",            vim_cmds              },
    { "gcc",             gcc_cmds              },
    { "g++",             gcc_cmds              },
    { "python",          python_cmds           },
    { "python3",         python_cmds           },
    { "gdb",             gdb_cmds              },
    { "valgrind",        valgrind_cmds         },
    { "ffmpeg",          ffmpeg_cmds           },
    { "rsync",           rsync_cmds            },
    { "ssh",             ssh_cmds              },
    { NULL, NULL }
};

char **get_subcommands(const char *cmd, const char *word, int *count_out) {
    *count_out = 0;
    if (!cmd) return NULL;

    /* find matching table entry */
    const char **subcmds = NULL;
    for (int i = 0; cmd_table[i].cmd; i++) {
        if (strcmp(cmd_table[i].cmd, cmd) == 0) {
            subcmds = cmd_table[i].subcmds;
            break;
        }
    }
    if (!subcmds) return NULL;

    /* count matches */
    int wlen = word ? strlen(word) : 0;
    int total = 0;
    for (int i = 0; subcmds[i]; i++) {
        if (wlen == 0 || strncmp(subcmds[i], word, wlen) == 0)
            total++;
    }
    if (total == 0) return NULL;

    char **results = malloc(total * sizeof(char *));
    if (!results) return NULL;

    int count = 0;
    for (int i = 0; subcmds[i] && count < total; i++) {
        if (wlen == 0 || strncmp(subcmds[i], word, wlen) == 0)
            results[count++] = strdup(subcmds[i]);
    }

    *count_out = count;
    return results;
}

const GITHUB_TOKEN = process.env.GITHUB_TOKEN;

const HEADERS = GITHUB_TOKEN
  ? {
      Authorization: `Bearer ${GITHUB_TOKEN}`,
    }
  : undefined;

export const getLatestRelease = async () => {
  const res = await fetch(
    "https://api.github.com/repos/arlyon/litehouse/releases/latest",
    { headers: HEADERS },
  );

  const data: {
    name: string;
    tag_name: string;
    html_url: string;
    body: string;
    published_at: string;
  } = await res.json();

  return data;
};

export const starsAndCommits = async (repoName: string) => {
  const response = await fetch(`https://api.github.com/repos/${repoName}`, {
    headers: HEADERS,
    next: { revalidate: 86400 },
  }).then((res) => res.json());
  const stars = response.stargazers_count;

  let commits = undefined;
  if (response.commits_url) {
    const response2 = await fetch(response.commits_url?.replace("{/sha}", ""), {
      headers: HEADERS,
      next: { revalidate: 86400 },
    });
    commits = await response2.json();
  }

  const commit = commits?.[0]
    ? {
        sha: commits[0].sha,
        date: new Date(commits[0].commit.author.date),
        url: commits[0].html_url,
      }
    : undefined;

  return { stars, commit };
};

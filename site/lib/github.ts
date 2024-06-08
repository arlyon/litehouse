export const getLatestRelease = async () => {
  const res = await fetch(
    "https://api.github.com/repos/arlyon/litehouse/releases/latest",
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

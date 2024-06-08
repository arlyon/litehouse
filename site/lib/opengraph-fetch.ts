import { decode } from "html-entities";

export const metaTags = {
  title: "title",
  description: "description",
  // Basic metadata
  ogTitle: "og:title",
  ogType: "og:type",
  ogImage: "og:image",
  ogUrl: "og:url",
  // Optional metadata
  ogAudio: "og:audio",
  ogDescription: "og:description",
  ogDeterminer: "og:determiner",
  ogLocale: "og:locale",
  ogLocaleAlternate: "og:locale:alternate",
  ogSiteName: "og:site_name",
  ogVideo: "og:video",
  // Structured Properties
  //   images
  ogImageUrl: "og:image:url",
  ogImageSecureUrl: "og:image:secure_url",
  ogImageType: "og:image:type",
  ogImageWidth: "og:image:width",
  ogImageHeight: "og:image:height",
  ogImageAlt: "og:image:alt",
  //   video
  ogVideoSecureUrl: "og:video:secure_url",
  ogVideoType: "og:video:type",
  ogVideoWidth: "og:video:width",
  ogVideoHeight: "og:video:height",
  ogVideoUrl: "og:video:url",
  //   audio
  ogAudioSecureUrl: "og:audio:secure_url",
  ogAudioType: "og:audio:type",
  // Social Networks
  twitterPlayer: "twitter:player",
  twitterPlayerWidth: "twitter:player:width",
  twitterPlayerHeight: "twitter:player:height",
  twitterPlayerStream: "twitter:player:stream",
  twitterCard: "twitter:card",
  twitterDomain: "twitter:domain",
  twitterUrl: "twitter:url",
  twitterTitle: "twitter:title",
  twitterDescription: "twitter:description",
  twitterImage: "twitter:image",
  // No vertical
  //   article
  articlePublishedTime: "article:published_time",
  articleModifiedTime: "article:modified_time",
  articleExpirationTime: "article:expiration_time",
  articleAuthor: "article:author",
  articleSection: "article:section",
  articleTag: "article:tag",
  //   book
  bookAuthor: "book:author",
  bookIsbn: "book:isbn",
  bookReleaseDate: "book:release_date",
  bookTag: "book:tag",
  //   profile
  profileFirstName: "profile:first_name",
  profileLastName: "profile:last_name",
  profileUsername: "profile:username",
  profileGender: "profile:gender",
};

export const queryParams = (str: string) => {
  const url = str.replace(/^([^#]*).*/, "$1").replace(/^[^?]*\??(.*)/, "$1");
  let result = {};
  const regex = /([^=]+)=([^&]+)&?/g;
  let match: RegExpExecArray | null;
  do {
    match = regex.exec(url);
    if (match) {
      if (match.index === regex.lastIndex) regex.lastIndex++;
      result = { ...result, [match[1]]: match[2] };
    }
  } while (match);

  return result;
};

export const fetchRaw = async (url: string, headers = {}) => {
  try {
    const response = await fetch(url.replace(/^([^?#]*).*/, "$1"), {
      method: "GET",
      headers: {
        "User-Agent": "OpenGraph",
        "Cache-Control": "no-cache",
        Accept: "*/*",
        Connection: "keep-alive",
        ...headers,
      },
    });

    if (!response.ok) {
      throw new Error(`HTTP error! Status: ${response.status}`);
    }

    const data = await response.text();
    return data;
  } catch (error) {
    throw { message: error.message };
  }
};

export const ogFetch = async (
  url: string,
  headers = {},
  includeRaw = false,
) => {
  const {
    title,
    description,
    ogUrl,
    ogType,
    ogTitle,
    ogDescription,
    ogImage,
    ogVideo,
    ogVideoType,
    ogVideoWidth,
    ogVideoHeight,
    ogVideoUrl,
    twitterPlayer,
    twitterPlayerWidth,
    twitterPlayerHeight,
    twitterPlayerStream,
    twitterCard,
    twitterDomain,
    twitterUrl,
    twitterTitle,
    twitterDescription,
    twitterImage,
  } = metaTags;

  try {
    const html = await fetchRaw(url, headers);
    let siteTitle = "";

    const tagTitle = html.match(
      /<title[^>]*>[\r\n\t\s]*([^<]+)[\r\n\t\s]*<\/title>/gim,
    );
    siteTitle = tagTitle
      ? tagTitle[0].replace(
          /<title[^>]*>[\r\n\t\s]*([^<]+)[\r\n\t\s]*<\/title>/gim,
          "$1",
        )
      : "";

    const og = [];
    const metas = html.match(/<meta[^>]+>/gim);

    if (metas) {
      for (let meta of metas) {
        meta = meta.replace(/\s*\/?>$/, " />");
        const zname = meta.replace(
          /[\s\S]*(property|name)\s*=\s*([\s\S]+)/,
          "$2",
        );
        const name = /^["']/.test(zname)
          ? zname.substr(1, zname.slice(1).indexOf(zname[0]))
          : zname.substr(0, zname.search(/[\s\t]/g));
        const valid = !!Object.keys(metaTags).filter(
          (m) => metaTags[m].toLowerCase() === name.toLowerCase(),
        ).length;

        if (valid) {
          const zcontent = meta.replace(
            /[\s\S]*(content)\s*=\s*([\s\S]+)/,
            "$2",
          );
          const content = /^["']/.test(zcontent)
            ? zcontent.substr(1, zcontent.slice(1).indexOf(zcontent[0]))
            : zcontent.substr(0, zcontent.search(/[\s\t]/g));
          og.push({ name, value: content !== "undefined" ? content : null });
        }
      }
    }

    const result = og.reduce(
      (chain, meta) => {
        chain[meta.name] = meta.value;
        return chain;
      },
      {
        url,
        raw: includeRaw ? html : null,
      },
    );

    // Image
    result[ogImage] = result[ogImage] ? result[ogImage] : null;

    result[twitterImage] = result[twitterImage]
      ? result[twitterImage]
      : result[ogImage];

    result.image = result[ogImage] ? result[ogImage] : null;

    // Video
    result.video = result[ogVideo]
      ? result[ogVideo]
      : result[ogVideoUrl]
        ? result[ogVideoUrl]
        : null;
    if (result.video) {
      result[ogVideoWidth] = result[ogVideoWidth] ? result[ogVideoWidth] : 560;
      result[ogVideoHeight] = result[ogVideoHeight]
        ? result[ogVideoHeight]
        : 340;
    }

    // URL
    result[ogUrl] = result[ogUrl] ? result[ogUrl] : url;

    result[twitterUrl] = result[twitterUrl]
      ? result[twitterUrl]
      : result[ogUrl];

    result.url = url;

    // Description
    result[ogDescription] = result[ogDescription]
      ? result[ogDescription]
      : result.description;

    result[twitterDescription] = result[twitterDescription]
      ? result[twitterDescription]
      : result[ogDescription];

    result.description = result[ogDescription];

    // Title
    result[ogTitle] = result[ogTitle] ? result[ogTitle] : siteTitle;

    result[twitterTitle] = result[twitterTitle]
      ? result[twitterTitle]
      : result[ogTitle];

    result.title = result[ogTitle];

    // Type
    result[ogType] = result[ogType] ? result[ogType] : "website";

    return result;
  } catch (error) {
    return {
      message: error.message,
      status: error.status || 400,
      error,
      [title]: "",
      [description]: "",
      [ogUrl]: url,
      [ogType]: "website",
      [ogTitle]: "",
      [ogDescription]: "",
      [ogImage]: "",
      [twitterCard]: "",
      [twitterDomain]: "",
      [twitterUrl]: url,
      [twitterTitle]: "",
      [twitterDescription]: "",
      [twitterImage]: "",
    };
  }
};

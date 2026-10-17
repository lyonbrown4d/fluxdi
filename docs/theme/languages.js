(() => {
    const current = new URL(window.location.href);
    const locale = current.pathname.match(/\/(en|zh-CN)\//);
    const menu = document.querySelector(".menu-bar .right-buttons");
    if (!locale || !menu) return;

    const navigation = document.createElement("nav");
    navigation.className = "language-navigation";
    navigation.setAttribute("aria-label", "Language / 文档语言");
    for (const [language, label] of [["en", "English"], ["zh-CN", "简体中文"]]) {
        const target = new URL(current.href);
        target.pathname = current.pathname.replace(/\/(en|zh-CN)\//, `/${language}/`);
        target.hash = "";
        const link = document.createElement("a");
        link.href = target.href;
        link.lang = language;
        link.hreflang = language;
        link.textContent = label;
        if (language === locale[1]) link.setAttribute("aria-current", "page");
        navigation.append(link);
    }
    menu.append(navigation);
})();
